// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

mod common;

use std::sync::{Arc, Mutex};

use common::{
    ArcEnumerator, MockEnumerator, MockHttp, NoopSleeper, SAMPLE_CATALOG_JSON, fast_update_config,
    make_device, make_official_firmware_image_bytes, wait_until,
};
use dualsense_core::{
    BatteryChargingState, CoreError, DEFAULT_PID, FirmwareImage, REPORT_ID_UPDATE_COMMAND,
};
use dualsense_ui::{
    AppEvent, AppRuntime, AppStep, BannerSeverity, NoFlashOutcome, ProgressStage,
    SIMULATED_DEVICE_PATH, SIMULATED_INITIAL_FW_VERSION, SIMULATED_SERIAL_NUMBER, UiSnapshot,
};

#[test]
fn no_flash_success_headless_dummy_end_to_end_and_rescan_updates_version() {
    let image_bytes = make_official_firmware_image_bytes(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES + 256,
        DEFAULT_PID,
        0x000B,
        0x0630,
    );
    let enum_mock = MockEnumerator::default();
    enum_mock.set_list_result(Err(CoreError::Hid("HID permission denied".to_string())));

    let http_mock = MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, image_bytes);
    let recorded = Arc::new(Mutex::new(Vec::<UiSnapshot>::new()));
    let recorded_cb = Arc::clone(&recorded);

    let runtime = AppRuntime::with_backends_and_no_flash(
        enum_mock,
        http_mock,
        NoopSleeper,
        fast_update_config(),
        Some(NoFlashOutcome::Success),
        move |snap| {
            recorded_cb.lock().expect("lock recorded").push(snap);
        },
    )
    .expect("start simulated runtime");

    assert!(runtime.snapshot().no_flash_active);

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    let after_scan = wait_until(&mut rx, |s| {
        !s.is_busy
            && s.selected_device.as_ref().and_then(|d| d.firmware_version)
                == Some(SIMULATED_INITIAL_FW_VERSION)
    });
    assert!(after_scan.no_flash_active);
    assert_eq!(
        after_scan.selected_device_path.as_deref(),
        Some(SIMULATED_DEVICE_PATH)
    );
    assert_eq!(after_scan.board_model, "BDM-050");
    assert_eq!(after_scan.detected_target_id.as_deref(), Some("000B"));
    assert_eq!(
        after_scan.serial_number.as_deref(),
        Some(SIMULATED_SERIAL_NUMBER)
    );
    assert_eq!(after_scan.color_name.as_deref(), Some("Midnight Black"));
    assert!(after_scan.battery_summary.is_some());
    assert!(!after_scan.battery_is_low);

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    let after_catalog = wait_until(&mut rx, |s| {
        s.step == AppStep::SelectFirmware && !s.is_busy && s.catalog_checked
    });
    assert_eq!(after_catalog.selected_target_id, "000B");
    assert_eq!(after_catalog.update_available, Some(true));
    assert!(after_catalog.can_review_update);

    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let completed = wait_until(&mut rx, |s| s.step == AppStep::Completed);
    assert!(completed.no_flash_active);
    assert_eq!(completed.completed_version.as_deref(), Some("0x0630"));
    assert!(completed.post_flash_verified);
    assert!(completed.has_cached_firmware);
    assert_eq!(
        completed.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );
    let report = completed
        .flash_report
        .as_ref()
        .expect("flash_report generated on simulated success");
    assert!(report.succeeded);
    assert!(report.markdown.contains("0x0400"));
    assert!(report.markdown.contains("0x0630"));

    {
        let history = recorded.lock().expect("lock history");
        assert!(
            history
                .iter()
                .any(|s| s.progress.stage == ProgressStage::Downloading)
        );
        assert!(
            history
                .iter()
                .any(|s| s.progress.stage == ProgressStage::StartingUpdate)
        );
        assert!(
            history
                .iter()
                .any(|s| s.progress.stage == ProgressStage::WritingFirmware
                    && s.progress.percentage == Some(100))
        );
        assert!(
            history
                .iter()
                .any(|s| s.progress.stage == ProgressStage::VerifyingFirmware)
        );
        assert!(
            history
                .iter()
                .any(|s| s.progress.stage == ProgressStage::FinalizingUpdate)
        );
    }

    // Re-scanning the dummy controller reports the updated 0x0630 firmware version.
    runtime.dispatch(AppEvent::ScanDevices);
    let after_rescan = wait_until(&mut rx, |s| {
        s.step == AppStep::DetectController
            && !s.is_busy
            && s.selected_device.as_ref().and_then(|d| d.firmware_version) == Some(0x0630)
    });
    assert!(after_rescan.is_up_to_date);
    runtime.shutdown();
}

#[test]
fn no_flash_error_local_file_reaches_error_step_and_builds_flash_report() {
    let image_bytes = make_official_firmware_image_bytes(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DEFAULT_PID,
        0x000B,
        0x0630,
    );
    let temp_path = std::env::temp_dir().join(format!(
        "dualsense_ui_no_flash_err_{}.bin",
        std::process::id()
    ));
    std::fs::write(&temp_path, &image_bytes).expect("write temp bin");

    let runtime = AppRuntime::with_backends_and_no_flash(
        MockEnumerator::default(),
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        Some(NoFlashOutcome::Error),
        |_| {},
    )
    .expect("start simulated error runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx, |s| {
        !s.is_busy
            && s.selected_device.as_ref().and_then(|d| d.firmware_version)
                == Some(SIMULATED_INITIAL_FW_VERSION)
    });

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    wait_until(&mut rx, |s| s.step == AppStep::SelectFirmware && !s.is_busy);

    runtime.dispatch(AppEvent::SelectLocalFile {
        path: temp_path.clone(),
    });
    wait_until(&mut rx, |s| {
        !s.is_busy && s.local_firmware.is_some() && s.can_review_update
    });

    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let err_snap = wait_until(&mut rx, |s| s.step == AppStep::Error);
    assert!(err_snap.no_flash_active);
    assert!(err_snap.error.is_some());
    assert!(!err_snap.error_title.is_empty());
    assert!(!err_snap.recovery_hint.is_empty());

    let report = err_snap
        .flash_report
        .as_ref()
        .expect("flash_report generated on simulated error");
    assert!(!report.succeeded);
    assert_eq!(report.failure_stage.as_deref(), Some("Flashing (USB HID)"));
    assert!(report.markdown.contains("VERIFY_FW_BODY_CMAC_CHECK_ERROR"));

    runtime.shutdown();
    let _ = std::fs::remove_file(&temp_path);
}

#[test]
fn no_flash_with_real_usb_controller_inspects_real_device_and_never_sends_f4() {
    let image_bytes = make_official_firmware_image_bytes(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DEFAULT_PID,
        0x0004,
        0x0630,
    );
    let enum_mock = Arc::new(MockEnumerator::with_devices(vec![make_device(
        "/dev/hidraw2",
        DEFAULT_PID,
        "REAL02000000002",
    )]));
    enum_mock.push_firmware_info("/dev/hidraw2", 0x0520);
    enum_mock.set_serial_number("/dev/hidraw2", "REAL02000000002");
    enum_mock.set_battery_report("/dev/hidraw2", 6, BatteryChargingState::Discharging);

    let runtime = AppRuntime::with_backends_and_no_flash(
        ArcEnumerator(Arc::clone(&enum_mock)),
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, image_bytes),
        NoopSleeper,
        fast_update_config(),
        Some(NoFlashOutcome::Success),
        |_| {},
    )
    .expect("start simulated runtime with real device");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    let after_scan = wait_until(&mut rx, |s| {
        !s.is_busy && s.selected_device.as_ref().and_then(|d| d.firmware_version) == Some(0x0520)
    });
    assert_eq!(
        after_scan.selected_device_path.as_deref(),
        Some("/dev/hidraw2")
    );
    assert_eq!(after_scan.color_name.as_deref(), Some("Cosmic Red"));
    assert_eq!(
        after_scan.battery_summary.as_deref(),
        Some("60% (Discharging)")
    );

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    wait_until(&mut rx, |s| {
        s.step == AppStep::SelectFirmware && !s.is_busy && s.catalog_checked
    });
    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let completed = wait_until(&mut rx, |s| s.step == AppStep::Completed);
    assert_eq!(completed.completed_version.as_deref(), Some("0x0630"));

    let sent = enum_mock.sent_reports("/dev/hidraw2");
    assert!(
        sent.iter()
            .all(|pkt| pkt.first().copied() != Some(REPORT_ID_UPDATE_COMMAND)),
        "0xF4 update commands must never be sent to real hardware in --no-flash mode"
    );
    runtime.shutdown();
}

#[test]
fn no_flash_none_preserves_normal_update_path() {
    let image_bytes = make_official_firmware_image_bytes(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DEFAULT_PID,
        0x0004,
        0x0630,
    );
    let enum_mock = Arc::new(MockEnumerator::with_devices(vec![make_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "NORM001",
    )]));
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);
    enum_mock.push_full_update_success("/dev/hidraw0", image_bytes.len());

    let runtime = AppRuntime::with_backends_and_no_flash(
        ArcEnumerator(Arc::clone(&enum_mock)),
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, image_bytes),
        NoopSleeper,
        fast_update_config(),
        None,
        |_| {},
    )
    .expect("start normal runtime");

    assert!(!runtime.snapshot().no_flash_active);
    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx, |s| {
        !s.is_busy && s.selected_device.as_ref().and_then(|d| d.firmware_version) == Some(0x0520)
    });
    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    wait_until(&mut rx, |s| {
        s.step == AppStep::SelectFirmware && !s.is_busy && s.catalog_checked
    });
    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    wait_until(&mut rx, |s| s.step == AppStep::Completed);

    let sent = enum_mock.sent_reports("/dev/hidraw0");
    assert!(
        sent.iter()
            .any(|pkt| pkt.first().copied() == Some(REPORT_ID_UPDATE_COMMAND)),
        "normal mode (no_flash: None) must send 0xF4 update commands"
    );
    runtime.shutdown();
}
