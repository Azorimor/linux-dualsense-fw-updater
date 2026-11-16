use std::io::Read;
use std::time::Duration;

use dualsense_core::{CancellationToken, CoreError, DownloadProgress, HttpClient, Result};
use ureq::Agent;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(20);
const DEFAULT_PER_READ_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_MAX_TEXT_BYTES: u64 = 1024 * 1024;
const DEFAULT_MAX_DOWNLOAD_BYTES: u64 = 64 * 1024 * 1024;
const DEFAULT_CHUNK_SIZE: usize = 16 * 1024;

/// Configuration for [`UreqHttpClient`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpClientConfig {
    pub connect_timeout: Duration,
    pub response_timeout: Duration,
    pub per_read_timeout: Duration,
    pub max_text_bytes: u64,
    pub max_download_bytes: u64,
    pub chunk_size: usize,
    pub user_agent: String,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT,
            per_read_timeout: DEFAULT_PER_READ_TIMEOUT,
            max_text_bytes: DEFAULT_MAX_TEXT_BYTES,
            max_download_bytes: DEFAULT_MAX_DOWNLOAD_BYTES,
            chunk_size: DEFAULT_CHUNK_SIZE,
            user_agent: format!("dualsense-updater/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

/// Synchronous HTTPS client backed by `ureq` + `rustls` with streaming progress and cancellation.
#[derive(Debug, Clone)]
pub struct UreqHttpClient {
    agent: Agent,
    config: HttpClientConfig,
}

impl Default for UreqHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl UreqHttpClient {
    /// Creates an HTTP client with default timeouts and rustls TLS verification.
    pub fn new() -> Self {
        Self::with_config(HttpClientConfig::default())
    }

    /// Creates an HTTP client with custom timeouts, size limits, and user-agent settings.
    pub fn with_config(config: HttpClientConfig) -> Self {
        let agent_config = Agent::config_builder()
            .timeout_connect(Some(config.connect_timeout))
            .timeout_recv_response(Some(config.response_timeout))
            .timeout_per_read(Some(config.per_read_timeout))
            .user_agent(config.user_agent.clone())
            .build();
        Self {
            agent: agent_config.into(),
            config,
        }
    }

    fn read_stream_with_cancel(
        &self,
        url: &str,
        max_bytes: u64,
        cancel: &CancellationToken,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<Vec<u8>> {
        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }

        let mut response = self.agent.get(url).call().map_err(|err| {
            log::warn!("HTTP request failed for {url}: {err}");
            CoreError::CatalogHttp(describe_ureq_error(&err))
        })?;

        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }

        let total_bytes = response
            .body()
            .content_length()
            .and_then(|len| usize::try_from(len).ok());

        if let Some(len) = total_bytes
            && (len as u64) > max_bytes
        {
            return Err(CoreError::CatalogHttp(
                "The server response exceeded the maximum allowed download size.".to_string(),
            ));
        }

        let mut bytes = Vec::with_capacity(total_bytes.unwrap_or(0).min(1024 * 1024));
        on_progress(DownloadProgress {
            downloaded_bytes: 0,
            total_bytes,
        });

        let chunk_len = self.config.chunk_size.max(1024);
        let mut buf = vec![0u8; chunk_len];
        let mut reader = response.body_mut().with_config().limit(max_bytes).reader();

        loop {
            if cancel.is_cancelled() {
                return Err(CoreError::Cancelled);
            }

            let read = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(err) => {
                    if cancel.is_cancelled() {
                        return Err(CoreError::Cancelled);
                    }
                    log::warn!("HTTP stream read failed for {url}: {err}");
                    return Err(CoreError::CatalogHttp(describe_stream_io_error(&err)));
                }
            };

            bytes.extend_from_slice(&buf[..read]);
            on_progress(DownloadProgress {
                downloaded_bytes: bytes.len(),
                total_bytes,
            });
        }

        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }

        Ok(bytes)
    }
}

impl HttpClient for UreqHttpClient {
    fn get_text(&self, url: &str, cancel: &CancellationToken) -> Result<String> {
        let mut noop = |_| {};
        let bytes =
            self.read_stream_with_cancel(url, self.config.max_text_bytes, cancel, &mut noop)?;
        String::from_utf8(bytes).map_err(|_| {
            CoreError::CatalogParse(
                "The update server returned text that could not be decoded as UTF-8.".to_string(),
            )
        })
    }

    fn download_with_progress(
        &self,
        url: &str,
        cancel: &CancellationToken,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<Vec<u8>> {
        self.read_stream_with_cancel(url, self.config.max_download_bytes, cancel, on_progress)
    }
}

/// Converts a `ureq::Error` into a plain-English network error message safe for non-technical users.
pub fn describe_ureq_error(err: &ureq::Error) -> String {
    match err {
        ureq::Error::HostNotFound => {
            "Could not find Sony's update server. Please check your internet connection and DNS settings."
                .to_string()
        }
        ureq::Error::Timeout(_) => {
            "The connection to Sony's update server timed out. Please check your internet connection and try again."
                .to_string()
        }
        ureq::Error::StatusCode(404) => {
            "The requested firmware update could not be found on Sony's server."
                .to_string()
        }
        ureq::Error::StatusCode(403 | 401) => {
            "Access to Sony's update server was denied."
                .to_string()
        }
        ureq::Error::StatusCode(500..=599) => {
            "Sony's firmware server is temporarily unavailable. Please try again in a few minutes."
                .to_string()
        }
        ureq::Error::StatusCode(code) => {
            format!("Sony's update server returned an unexpected response (HTTP {code}).")
        }
        ureq::Error::Tls(_) | ureq::Error::Rustls(_) | ureq::Error::Pem(_) => {
            "Could not establish a secure HTTPS connection to Sony's update server."
                .to_string()
        }
        ureq::Error::ConnectionFailed => {
            "Could not connect to Sony's update server. Please check your internet connection."
                .to_string()
        }
        ureq::Error::Io(io_err) => describe_io_kind(io_err.kind()),
        ureq::Error::BadUri(_) | ureq::Error::InvalidProxyUrl => {
            "The update download address or proxy configuration is invalid."
                .to_string()
        }
        ureq::Error::TooManyRedirects | ureq::Error::RedirectFailed => {
            "The update server redirected the request too many times."
                .to_string()
        }
        ureq::Error::Protocol(_) | ureq::Error::Http(_) => {
            "Received an unexpected network response from the update server."
                .to_string()
        }
        _ => "Could not complete the download from Sony's update server.".to_string(),
    }
}

fn describe_stream_io_error(err: &std::io::Error) -> String {
    if let Some(ureq_err) = err
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<ureq::Error>())
    {
        return describe_ureq_error(ureq_err);
    }
    describe_io_kind(err.kind())
}

fn describe_io_kind(kind: std::io::ErrorKind) -> String {
    match kind {
        std::io::ErrorKind::TimedOut => {
            "The download timed out before finishing. Please check your internet connection and try again."
                .to_string()
        }
        std::io::ErrorKind::ConnectionRefused
        | std::io::ErrorKind::ConnectionReset
        | std::io::ErrorKind::ConnectionAborted
        | std::io::ErrorKind::NotConnected => {
            "The connection to the update server was interrupted. Please check your internet connection and try again."
                .to_string()
        }
        std::io::ErrorKind::UnexpectedEof => {
            "The download ended unexpectedly before the entire file was received."
                .to_string()
        }
        _ => "A network error interrupted the download. Please check your internet connection and try again.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    fn spawn_http_server<F>(handler: F) -> (String, thread::JoinHandle<()>)
    where
        F: FnOnce(std::net::TcpStream) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept client");
            handler(stream);
        });
        (format!("http://{addr}"), handle)
    }

    fn drain_request(stream: &mut std::net::TcpStream) {
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
    }

    #[test]
    fn get_text_fetches_utf8_payload_from_local_server() {
        let body = r#"{"ApplicationLatestVersion":"2.2.1.2","FwUpdate0004LatestVersion":"0x0630"}"#;
        let (base_url, handle) = spawn_http_server(move |mut stream| {
            drain_request(&mut stream);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });

        let client = UreqHttpClient::new();
        let cancel = CancellationToken::new();
        let text = client
            .get_text(&format!("{base_url}/info.json"), &cancel)
            .expect("get_text succeeds");
        assert_eq!(text, body);
        handle.join().expect("server thread");
    }

    #[test]
    fn download_with_progress_streams_chunks_and_reports_content_length() {
        let payload = vec![0xABu8; 4096];
        let payload_len = payload.len();
        let (base_url, handle) = spawn_http_server(move |mut stream| {
            drain_request(&mut stream);
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {payload_len}\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(headers.as_bytes()).expect("write headers");
            stream.write_all(&payload[..1500]).expect("write part 1");
            stream.flush().expect("flush part 1");
            thread::sleep(Duration::from_millis(10));
            stream.write_all(&payload[1500..]).expect("write part 2");
        });

        let config = HttpClientConfig {
            chunk_size: 1024,
            ..Default::default()
        };
        let client = UreqHttpClient::with_config(config);
        let cancel = CancellationToken::new();

        let mut events = Vec::new();
        let bytes = client
            .download_with_progress(&format!("{base_url}/fw.bin"), &cancel, &mut |p| {
                events.push(p);
            })
            .expect("download succeeds");

        assert_eq!(bytes.len(), 4096);
        assert!(bytes.iter().all(|b| *b == 0xAB));
        assert!(events.len() >= 2);
        assert_eq!(
            events.first().copied(),
            Some(DownloadProgress {
                downloaded_bytes: 0,
                total_bytes: Some(4096),
            })
        );
        assert_eq!(
            events.last().copied(),
            Some(DownloadProgress {
                downloaded_bytes: 4096,
                total_bytes: Some(4096),
            })
        );
        handle.join().expect("server thread");
    }

    #[test]
    fn respects_cancellation_before_and_during_streaming_download() {
        let client = UreqHttpClient::new();
        let precancel = CancellationToken::new();
        precancel.cancel();
        assert_eq!(
            client.get_text("http://127.0.0.1:9/unused", &precancel),
            Err(CoreError::Cancelled)
        );

        let (base_url, handle) = spawn_http_server(move |mut stream| {
            drain_request(&mut stream);
            let headers = "HTTP/1.1 200 OK\r\nContent-Length: 8192\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(headers.as_bytes());
            let _ = stream.write_all(&[0x55u8; 2048]);
            let _ = stream.flush();
            thread::sleep(Duration::from_millis(120));
            let _ = stream.write_all(&[0x55u8; 6144]);
        });

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        let res = client.download_with_progress(&format!("{base_url}/fw.bin"), &cancel, &mut |p| {
            if p.downloaded_bytes > 0 {
                cancel_clone.cancel();
            }
        });
        assert_eq!(res, Err(CoreError::Cancelled));
        handle.join().expect("server thread");
    }

    #[test]
    fn maps_http_status_and_network_errors_to_plain_english() {
        let (base_url, handle) = spawn_http_server(move |mut stream| {
            drain_request(&mut stream);
            let response =
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream.write_all(response.as_bytes()).expect("write 404");
        });

        let client = UreqHttpClient::new();
        let cancel = CancellationToken::new();
        let err = client
            .get_text(&format!("{base_url}/missing.bin"), &cancel)
            .expect_err("404 should fail");
        match err {
            CoreError::CatalogHttp(msg) => assert!(msg.contains("could not be found")),
            other => panic!("unexpected error variant: {other:?}"),
        }
        handle.join().expect("server thread");

        assert!(describe_ureq_error(&ureq::Error::HostNotFound).contains("internet connection"));
        assert!(
            describe_ureq_error(&ureq::Error::StatusCode(503)).contains("temporarily unavailable")
        );
        assert!(describe_ureq_error(&ureq::Error::StatusCode(403)).contains("denied"));
        assert!(
            describe_ureq_error(&ureq::Error::Timeout(ureq::Timeout::Global)).contains("timed out")
        );
        assert!(describe_ureq_error(&ureq::Error::Tls("handshake")).contains("secure HTTPS"));
        assert!(describe_ureq_error(&ureq::Error::ConnectionFailed).contains("Could not connect"));
        assert!(describe_io_kind(std::io::ErrorKind::UnexpectedEof).contains("ended unexpectedly"));
    }

    #[test]
    fn rejects_invalid_utf8_and_oversized_responses() {
        let (base_url, handle) = spawn_http_server(move |mut stream| {
            drain_request(&mut stream);
            let body = [0xFFu8, 0xFEu8, 0xFDu8];
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(headers.as_bytes()).expect("write headers");
            stream.write_all(&body).expect("write non-utf8");
        });

        let client = UreqHttpClient::new();
        let cancel = CancellationToken::new();
        let err = client
            .get_text(&format!("{base_url}/bad.txt"), &cancel)
            .expect_err("invalid utf8 should error");
        assert!(matches!(err, CoreError::CatalogParse(_)));
        handle.join().expect("server thread");

        let (base_url2, handle2) = spawn_http_server(move |mut stream| {
            drain_request(&mut stream);
            let headers = "HTTP/1.1 200 OK\r\nContent-Length: 999999\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(headers.as_bytes());
        });
        let config = HttpClientConfig {
            max_download_bytes: 1024,
            ..Default::default()
        };
        let bounded_client = UreqHttpClient::with_config(config);
        let err2 = bounded_client
            .download_with_progress(&format!("{base_url2}/huge.bin"), &cancel, &mut |_| {})
            .expect_err("oversized content-length should fail");
        assert!(matches!(err2, CoreError::CatalogHttp(_)));
        handle2.join().expect("server thread");
    }
}
