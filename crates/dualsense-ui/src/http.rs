// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

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

pub(crate) fn describe_io_kind(kind: std::io::ErrorKind) -> String {
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
