// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use dualsense_core::{CancellationToken, CoreError, DownloadProgress, HttpClient};

use crate::http::{HttpClientConfig, UreqHttpClient, describe_io_kind, describe_ureq_error};

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
        let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
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
    assert!(describe_ureq_error(&ureq::Error::StatusCode(503)).contains("temporarily unavailable"));
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
