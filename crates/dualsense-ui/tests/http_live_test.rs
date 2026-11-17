// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

mod common;

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use common::{
    ArcEnumerator, MockEnumerator, NoopSleeper, SAMPLE_CATALOG_JSON, fast_update_config,
    make_device, make_official_firmware_image_bytes, wait_until,
};
use dualsense_core::{DEFAULT_PID, FirmwareImage, REPORT_ID_UPDATE_COMMAND, UpdateCommand};
use dualsense_ui::{AppEvent, AppRuntime, AppStep, UreqHttpClient};

#[test]
fn end_to_end_live_ureq_http_client_against_local_server() {
    let image_bytes = make_official_firmware_image_bytes(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES + 128,
        DEFAULT_PID,
        0x0004,
        0x0630,
    );
    let image_len = image_bytes.len();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local server");
    let addr = listener.local_addr().expect("local addr");

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept download");
        let mut req_buf = [0u8; 512];
        let _ = stream.read(&mut req_buf);
        let headers =
            format!("HTTP/1.1 200 OK\r\nContent-Length: {image_len}\r\nConnection: close\r\n\r\n");
        stream.write_all(headers.as_bytes()).expect("write headers");
        stream.write_all(&image_bytes).expect("write firmware");
    });

    let enum_mock =
        MockEnumerator::with_devices(vec![make_device("/dev/hidraw0", DEFAULT_PID, "STD-01")]);
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);
    enum_mock.push_full_update_success("/dev/hidraw0", image_len);

    // Build a catalog pointing at our local TCP HTTP server so real UreqHttpClient downloads it.
    let mut local_catalog =
        dualsense_core::FirmwareCatalog::from_json(SAMPLE_CATALOG_JSON).expect("catalog");
    if let Some(item) = local_catalog
        .releases
        .iter_mut()
        .find(|r| r.target_id == "0004")
    {
        item.download_url = format!("http://{addr}/FWUPDATE0004.bin");
    }

    let runtime = AppRuntime::with_backends(
        enum_mock,
        UreqHttpClient::new(),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::CatalogLoaded(Ok(local_catalog)));
    wait_until(&mut rx, |s| s.can_review_update);

    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let done = wait_until(&mut rx, |s| s.step == AppStep::Completed);
    assert_eq!(done.completed_version.as_deref(), Some("0x0630"));

    server_handle.join().expect("server thread");
    runtime.shutdown();
}

#[test]
fn end_to_end_live_http_html_and_truncated_download_abort_before_flashing() {
    for (label, response) in [
        (
            "text/html error page",
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 45\r\nConnection: close\r\n\r\n<html><body>Service unavailable</body></html>"
                .to_string(),
        ),
        (
            "truncated Content-Length",
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
                "ABCD"
            ),
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local server");
        let addr = listener.local_addr().expect("local addr");
        let server_handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept download");
            let mut req_buf = [0u8; 512];
            let _ = stream.read(&mut req_buf);
            let _ = stream.write_all(response.as_bytes());
        });

        let enum_mock = Arc::new(MockEnumerator::with_devices(vec![make_device(
            "/dev/hidraw0",
            DEFAULT_PID,
            "STD-01",
        )]));
        enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);

        let mut local_catalog =
            dualsense_core::FirmwareCatalog::from_json(SAMPLE_CATALOG_JSON).expect("catalog");
        if let Some(item) = local_catalog
            .releases
            .iter_mut()
            .find(|r| r.target_id == "0004")
        {
            item.download_url = format!("http://{addr}/FWUPDATE0004.bin");
        }

        let runtime = AppRuntime::with_backends(
            ArcEnumerator(Arc::clone(&enum_mock)),
            UreqHttpClient::new(),
            NoopSleeper,
            fast_update_config(),
            |_| {},
        )
        .expect("start runtime");

        let mut rx = runtime.subscribe();
        runtime.dispatch(AppEvent::ScanDevices);
        wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);

        runtime.dispatch(AppEvent::ContinueToFirmwareStep);
        runtime.dispatch(AppEvent::CatalogLoaded(Ok(local_catalog)));
        wait_until(&mut rx, |s| s.can_review_update);

        runtime.dispatch(AppEvent::ContinueToConfirmStep);
        runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
        wait_until(&mut rx, |s| s.can_start_update);

        runtime.dispatch(AppEvent::StartUpdate);
        let err_snap = wait_until(&mut rx, |s| s.step == AppStep::Error);
        assert!(!err_snap.error_title.is_empty(), "{label}");
        assert!(!err_snap.error_message.is_empty(), "{label}");
        assert!(!err_snap.recovery_hint.is_empty(), "{label}");
        assert!(
            !err_snap.error_message.contains("ureq::"),
            "leaked ureq detail for {label}: {}",
            err_snap.error_message
        );

        let sent = enum_mock.sent_reports("/dev/hidraw0");
        assert!(
            !sent.iter().any(|r| {
                r.len() >= 2
                    && r[0] == REPORT_ID_UPDATE_COMMAND
                    && r[1] == UpdateCommand::StartUpdate as u8
            }),
            "StartUpdate must never be sent for {label}"
        );

        server_handle.join().expect("server thread");
        runtime.shutdown();
    }
}
