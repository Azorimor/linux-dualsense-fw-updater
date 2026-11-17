// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

mod common;

use std::sync::{Arc, Mutex};

use common::{
    ArcEnumerator, MockEnumerator, MockHttp, NoopSleeper, SAMPLE_CATALOG_JSON, fast_update_config,
    make_device, make_firmware_image_bytes, wait_until,
};
use dualsense_core::{
    CoreError, DEFAULT_PID, DUALSENSE_EDGE_PID, REPORT_ID_UPDATE_COMMAND, StartUpdateStatusCode,
    UpdateCommand, WRITE_BLOCK_SIZE,
};
use dualsense_ui::{AppEvent, AppRuntime, AppStep, BannerSeverity, ProgressStage, UiSnapshot};

#[test]
fn end_to_end_scan_catalog_download_and_update_succeeds() {
    let image_bytes = make_firmware_image_bytes(WRITE_BLOCK_SIZE + 128, 0x0630);
    let enum_mock = Arc::new(MockEnumerator::with_devices(vec![make_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "ABC123",
    )]));
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);
    enum_mock.push_full_update_success("/dev/hidraw0", image_bytes.len());

    let http_mock = MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, image_bytes.clone());

    let recorded = Arc::new(Mutex::new(Vec::<UiSnapshot>::new()));
    let recorded_cb = Arc::clone(&recorded);

    let runtime = AppRuntime::with_backends(
        ArcEnumerator(Arc::clone(&enum_mock)),
        http_mock,
        NoopSleeper,
        fast_update_config(),
        move |snap| {
            recorded_cb.lock().expect("lock recorded").push(snap);
        },
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    let after_scan = wait_until(&mut rx, |s| {
        !s.is_busy && s.selected_device.as_ref().and_then(|d| d.firmware_version) == Some(0x0520)
    });
    assert!(after_scan.can_continue_to_firmware);

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::CheckForUpdates);
    let after_catalog = wait_until(&mut rx, |s| !s.is_busy && s.catalog_checked);
    assert!(after_catalog.can_review_update);

    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let completed = wait_until(&mut rx, |s| s.step == AppStep::Completed);
    assert_eq!(completed.completed_version.as_deref(), Some("0x0630"));
    assert_eq!(completed.progress.percentage, Some(100));
    assert_eq!(
        completed.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );

    let history = recorded.lock().expect("lock history");
    assert!(
        history
            .iter()
            .any(|s| s.progress.status_text.starts_with("Downloading firmware…"))
    );
    assert!(
        history
            .iter()
            .any(|s| s.progress.status_text.starts_with("Installing firmware…"))
    );
    let sent = enum_mock.sent_reports("/dev/hidraw0");
    assert!(
        sent.iter()
            .any(|r| r[0] == REPORT_ID_UPDATE_COMMAND && r[1] == UpdateCommand::StartUpdate as u8)
    );
    assert!(
        sent.iter()
            .any(|r| r[0] == REPORT_ID_UPDATE_COMMAND
                && r[1] == UpdateCommand::WriteUpdateImage as u8)
    );
    assert!(sent.iter().any(
        |r| r[0] == REPORT_ID_UPDATE_COMMAND && r[1] == UpdateCommand::VerifyUpdateImage as u8
    ));
    assert_eq!(
        sent.last().map(|r| (r[0], r[1])),
        Some((
            REPORT_ID_UPDATE_COMMAND,
            UpdateCommand::FinalizeUpdate as u8
        ))
    );
    runtime.shutdown();
}

#[test]
fn end_to_end_multi_device_selection_and_local_file_update() {
    let image_bytes = make_firmware_image_bytes(1024, 0x0650);
    let temp_path = std::env::temp_dir().join(format!(
        "dualsense_ui_test_local_{}.bin",
        std::process::id()
    ));
    std::fs::write(&temp_path, &image_bytes).expect("write temp bin");

    let enum_mock = MockEnumerator::with_devices(vec![
        make_device("/dev/hidraw0", DEFAULT_PID, "STD-01"),
        make_device("/dev/hidraw1", DUALSENSE_EDGE_PID, "EDGE-02"),
    ]);
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);
    enum_mock.push_firmware_info_with_hw("/dev/hidraw1", 0x0210, 0x0044, 0x0000_0400);
    enum_mock.push_full_update_success("/dev/hidraw1", image_bytes.len());

    let runtime = AppRuntime::with_backends(
        enum_mock,
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx, |s| {
        !s.is_busy && s.selected_device.as_ref().and_then(|d| d.firmware_version) == Some(0x0520)
    });

    // Switch to second device (DualSense Edge)
    runtime.dispatch(AppEvent::SelectDevice {
        path: "/dev/hidraw1".to_string(),
    });
    let edge_ready = wait_until(&mut rx, |s| {
        !s.is_busy
            && s.selected_device_path.as_deref() == Some("/dev/hidraw1")
            && s.selected_device.as_ref().and_then(|d| d.firmware_version) == Some(0x0210)
    });
    assert_eq!(edge_ready.selected_target_id, "0044");

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::SelectLocalFile {
        path: temp_path.clone(),
    });
    let file_ready = wait_until(&mut rx, |s| !s.is_busy && s.local_firmware.is_some());
    assert!(file_ready.can_review_update);

    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let done = wait_until(&mut rx, |s| s.step == AppStep::Completed);
    assert_eq!(done.completed_version.as_deref(), Some("0x0650"));

    let _ = std::fs::remove_file(&temp_path);
    runtime.shutdown();
}

#[test]
fn end_to_end_download_cancellation_aborts_before_flashing() {
    let enum_mock =
        MockEnumerator::with_devices(vec![make_device("/dev/hidraw0", DEFAULT_PID, "STD-01")]);
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);

    let runtime = AppRuntime::with_backends(
        enum_mock,
        MockHttp::slow_cancellable(SAMPLE_CATALOG_JSON),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::CheckForUpdates);
    wait_until(&mut rx, |s| !s.is_busy && s.catalog_checked);

    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let downloading = wait_until(&mut rx, |s| {
        s.step == AppStep::Updating
            && s.progress.stage == ProgressStage::Downloading
            && s.can_cancel
    });
    assert!(downloading.can_cancel);

    runtime.dispatch(AppEvent::RequestCancel);
    let cancelled = wait_until(&mut rx, |s| s.step == AppStep::ConfirmUpdate && !s.is_busy);
    assert!(!cancelled.warning_acknowledged);
    assert_eq!(
        cancelled.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Info)
    );
    runtime.shutdown();
}

#[test]
fn end_to_end_failure_paths_surface_plain_english_and_recovery() {
    // Case A: USB permission failure on scan
    let enum_mock = MockEnumerator::default();
    enum_mock.set_list_result(Err(CoreError::Hid(
        "Permission denied opening /dev/hidraw0".to_string(),
    )));

    let runtime = AppRuntime::with_backends(
        enum_mock,
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    let err_snap = wait_until(&mut rx, |s| s.step == AppStep::Error);
    assert!(err_snap.error_title.contains("permission"));
    assert!(err_snap.recovery_hint.contains("udev"));
    assert!(err_snap.can_scan);

    runtime.dispatch(AppEvent::DismissError);
    let recovered = wait_until(&mut rx, |s| s.step == AppStep::DetectController);
    assert!(recovered.error_title.is_empty());
    runtime.shutdown();

    // Case B: Controller rejects StartUpdate (already running firmware)
    let enum_mock2 =
        MockEnumerator::with_devices(vec![make_device("/dev/hidraw0", DEFAULT_PID, "STD-01")]);
    enum_mock2.push_firmware_info("/dev/hidraw0", 0x0630);
    enum_mock2.push_status(
        "/dev/hidraw0",
        UpdateCommand::StartUpdate,
        StartUpdateStatusCode::HeaderVersionCheckError as u8,
    );

    let image_bytes = make_firmware_image_bytes(512, 0x0630);
    let runtime2 = AppRuntime::with_backends(
        enum_mock2,
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, image_bytes),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime 2");

    let mut rx2 = runtime2.subscribe();
    runtime2.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx2, |s| !s.is_busy && s.can_continue_to_firmware);
    runtime2.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime2.dispatch(AppEvent::CheckForUpdates);
    wait_until(&mut rx2, |s| !s.is_busy && s.catalog_checked);
    runtime2.dispatch(AppEvent::ContinueToConfirmStep);
    runtime2.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx2, |s| s.can_start_update);

    runtime2.dispatch(AppEvent::StartUpdate);
    let rejection = wait_until(&mut rx2, |s| s.step == AppStep::Error);
    assert!(rejection.error_title.contains("already has this"));
    assert!(!rejection.error_message.contains("HEADER_"));
    assert!(!rejection.recovery_hint.is_empty());
    runtime2.shutdown();
}

#[test]
fn end_to_end_bdm050_board_and_midnight_black_color_auto_selects_target() {
    let image_bytes = make_firmware_image_bytes(WRITE_BLOCK_SIZE + 64, 0x0630);
    let enum_mock =
        MockEnumerator::with_devices(vec![make_device("/dev/hidraw0", DEFAULT_PID, "")]);
    enum_mock.push_firmware_info_with_hw("/dev/hidraw0", 0x0520, 0x000B, 0x0000_0811);
    enum_mock.set_serial_number("/dev/hidraw0", "F55301K0H14359961");
    enum_mock.push_full_update_success("/dev/hidraw0", image_bytes.len());

    let http_mock = MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, image_bytes);
    let runtime = AppRuntime::with_backends(
        enum_mock,
        http_mock,
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    let after_scan = wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);

    assert_eq!(after_scan.board_model, "BDM-050");
    assert_eq!(after_scan.fw_series_text, "0x000B");
    assert_eq!(after_scan.hw_model_text, "0x00000811");
    assert_eq!(
        after_scan.serial_number.as_deref(),
        Some("F55301K0H14359961")
    );
    assert_eq!(after_scan.color_name.as_deref(), Some("Midnight Black"));
    assert_eq!(after_scan.detected_target_id.as_deref(), Some("000B"));
    assert!(after_scan.board_supported);
    assert!(after_scan.unsupported_board_warning.is_none());
    assert_eq!(after_scan.selected_target_id, "000B");
    assert!(after_scan.can_select_target);

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    let step2 = wait_until(&mut rx, |s| s.step == AppStep::SelectFirmware);
    assert!(step2.can_select_target);
    assert!(step2.can_check_updates);

    runtime.dispatch(AppEvent::CheckForUpdates);
    let after_catalog = wait_until(&mut rx, |s| !s.is_busy && s.catalog_checked);
    assert!(after_catalog.can_review_update);

    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let completed = wait_until(&mut rx, |s| s.step == AppStep::Completed);
    assert_eq!(completed.completed_version.as_deref(), Some("0x0630"));
    runtime.shutdown();
}

#[test]
fn end_to_end_unsupported_bdm010_and_missing_serial_behave_safely() {
    let enum_mock =
        MockEnumerator::with_devices(vec![make_device("/dev/hidraw0", DEFAULT_PID, "")]);
    enum_mock.push_firmware_info_with_hw("/dev/hidraw0", 0x0200, 0x0001, 0x0000_0300);

    let runtime = AppRuntime::with_backends(
        enum_mock,
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    let after_scan = wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);
    assert_eq!(after_scan.board_model, "BDM-010");
    assert_eq!(after_scan.fw_series_text, "0x0001");
    assert_eq!(after_scan.hw_model_text, "0x00000300");
    assert!(after_scan.serial_number.is_none());
    assert!(after_scan.color_name.is_none());
    assert!(after_scan.detected_target_id.is_none());
    assert!(!after_scan.board_supported);
    assert!(!after_scan.can_select_target);
    assert!(
        after_scan
            .unsupported_board_warning
            .as_deref()
            .is_some_and(|w| w.contains("BDM-010"))
    );

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    let step2 = wait_until(&mut rx, |s| s.step == AppStep::SelectFirmware);
    assert!(!step2.can_select_target);
    assert!(!step2.can_check_updates);
    assert!(step2.can_select_file);

    runtime.dispatch(AppEvent::SelectTarget {
        target_id: "0004".to_string(),
    });
    runtime.dispatch(AppEvent::CheckForUpdates);
    let guarded = runtime.snapshot();
    assert!(!guarded.is_busy);
    assert!(!guarded.catalog_checked);
    runtime.shutdown();

    // Unknown (zeroed) board on standard PID also disables target selection and update check
    let enum_mock_unknown =
        MockEnumerator::with_devices(vec![make_device("/dev/hidraw0", DEFAULT_PID, "")]);
    enum_mock_unknown.push_firmware_info_with_hw("/dev/hidraw0", 0x0500, 0x0000, 0x0000_0000);

    let runtime_unknown = AppRuntime::with_backends(
        enum_mock_unknown,
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime unknown");

    let mut rx_unknown = runtime_unknown.subscribe();
    runtime_unknown.dispatch(AppEvent::ScanDevices);
    let snap_unknown = wait_until(&mut rx_unknown, |s| {
        !s.is_busy && s.can_continue_to_firmware
    });
    assert_eq!(snap_unknown.board_model, "Unknown");
    assert!(!snap_unknown.board_supported);
    assert!(!snap_unknown.can_select_target);
    assert!(!snap_unknown.can_check_updates);
    assert!(snap_unknown.unsupported_board_warning.is_some());
    runtime_unknown.shutdown();
}
