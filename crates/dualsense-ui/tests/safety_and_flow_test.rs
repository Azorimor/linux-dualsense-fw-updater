// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::{
    ArcEnumerator, MockEnumerator, MockHttp, NoopSleeper, SAMPLE_CATALOG_JSON, fast_update_config,
    make_bluetooth_device, make_device, make_firmware_image_bytes,
    make_official_firmware_image_bytes, wait_until,
};
use dualsense_core::{
    BatteryChargingState, DEFAULT_PID, FirmwareImage, LocalImageCompatibility,
    REPORT_ID_UPDATE_COMMAND, UpdateCommand,
};
use dualsense_ui::{AppEvent, AppRuntime, AppStep, BannerSeverity};

#[test]
fn end_to_end_bluetooth_only_detection_and_usb_plug_recovery() {
    let enum_mock = Arc::new(MockEnumerator::with_devices(vec![make_bluetooth_device(
        "/dev/hidraw9",
        DEFAULT_PID,
        "BT-1",
    )]));

    let runtime = AppRuntime::with_backends(
        ArcEnumerator(Arc::clone(&enum_mock)),
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    let bt_snap = wait_until(&mut rx, |s| !s.is_busy && s.bluetooth_only_detected);
    assert_eq!(bt_snap.bluetooth_device_count, 1);
    assert!(bt_snap.devices.is_empty());
    assert!(!bt_snap.can_continue_to_firmware);
    assert_eq!(
        bt_snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Warning)
    );
    assert!(
        bt_snap
            .banner
            .as_ref()
            .is_some_and(|b| b.text.contains("USB data cable"))
    );

    // Plug in a USB controller alongside Bluetooth and rescan
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);
    enum_mock.set_list_result(Ok(vec![
        make_bluetooth_device("/dev/hidraw9", DEFAULT_PID, "BT-1"),
        make_device("/dev/hidraw0", DEFAULT_PID, "USB-1"),
    ]));
    runtime.dispatch(AppEvent::ScanDevices);
    let usb_snap = wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);
    assert!(!usb_snap.bluetooth_only_detected);
    assert_eq!(usb_snap.bluetooth_device_count, 1);
    assert_eq!(usb_snap.devices.len(), 1);
    assert_eq!(usb_snap.devices[0].path, "/dev/hidraw0");
    runtime.shutdown();
}

#[test]
fn end_to_end_incompatible_local_bin_blocked_before_flashing() {
    let enum_mock = Arc::new(MockEnumerator::with_devices(vec![make_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "STD-1",
    )]));
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0510);

    let wrong_target_bytes = make_official_firmware_image_bytes(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DEFAULT_PID,
        0x000B,
        0x0630,
    );
    let no_header_bytes = make_firmware_image_bytes(1024, 0x0630);
    let pid = std::process::id();
    let wrong_path = std::env::temp_dir().join(format!("ds_wrong_target_{pid}.bin"));
    let bad_hdr_path = std::env::temp_dir().join(format!("ds_no_hdr_{pid}.bin"));
    std::fs::write(&wrong_path, &wrong_target_bytes).expect("write wrong target");
    std::fs::write(&bad_hdr_path, &no_header_bytes).expect("write bad hdr");

    let runtime = AppRuntime::with_backends(
        ArcEnumerator(Arc::clone(&enum_mock)),
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);
    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    wait_until(&mut rx, |s| s.step == AppStep::SelectFirmware);

    // 1) Wrong target ID (000B file on 0004 controller) -> Incompatible
    runtime.dispatch(AppEvent::SelectLocalFile {
        path: wrong_path.clone(),
    });
    let wrong_snap = wait_until(&mut rx, |s| {
        !s.is_busy
            && s.local_firmware
                .as_ref()
                .is_some_and(|l| l.path == wrong_path)
    });
    assert_eq!(
        wrong_snap.local_firmware_compatibility,
        Some(LocalImageCompatibility::Incompatible)
    );
    assert!(!wrong_snap.local_firmware_compatible);
    assert!(!wrong_snap.can_review_update);
    assert!(wrong_snap.local_firmware_error.is_some());

    // 2) Missing Sony header -> Incompatible
    runtime.dispatch(AppEvent::SelectLocalFile {
        path: bad_hdr_path.clone(),
    });
    let hdr_snap = wait_until(&mut rx, |s| {
        !s.is_busy
            && s.local_firmware
                .as_ref()
                .is_some_and(|l| l.path == bad_hdr_path)
    });
    assert_eq!(
        hdr_snap.local_firmware_compatibility,
        Some(LocalImageCompatibility::Incompatible)
    );
    assert!(!hdr_snap.can_review_update);

    // Attempt to bypass step checks is ignored; no StartUpdate ever sent
    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    assert_eq!(runtime.snapshot().step, AppStep::SelectFirmware);
    assert!(
        !enum_mock
            .sent_reports("/dev/hidraw0")
            .iter()
            .any(|r| r.len() >= 2
                && r[0] == REPORT_ID_UPDATE_COMMAND
                && r[1] == UpdateCommand::StartUpdate as u8)
    );

    let _ = std::fs::remove_file(&wrong_path);
    let _ = std::fs::remove_file(&bad_hdr_path);
    runtime.shutdown();
}

#[test]
fn end_to_end_battery_check_low_and_absent_reports_work_cleanly() {
    let enum_mock = Arc::new(MockEnumerator::with_devices(vec![
        make_device("/dev/hidraw0", DEFAULT_PID, "LOW-BAT"),
        make_device("/dev/hidraw1", DEFAULT_PID, "NO-BAT"),
    ]));
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0510);
    enum_mock.set_battery_report("/dev/hidraw0", 1, BatteryChargingState::Discharging);
    enum_mock.push_firmware_info("/dev/hidraw1", 0x0510);

    let runtime = AppRuntime::with_backends(
        ArcEnumerator(Arc::clone(&enum_mock)),
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    let snap = wait_until(&mut rx, |s| {
        !s.is_busy
            && s.devices.len() == 2
            && s.devices.iter().all(|d| d.firmware_version.is_some())
            && s.catalog_checked
    });

    assert_eq!(snap.battery_percent, Some(10));
    assert_eq!(snap.battery_summary.as_deref(), Some("10% (Discharging)"));
    assert!(snap.battery_is_low);
    assert!(snap.battery_is_critically_low);
    assert!(snap.battery_warning.is_some());

    // Switch to controller without battery report -> inspection still succeeded cleanly
    runtime.dispatch(AppEvent::SelectDevice {
        path: "/dev/hidraw1".to_string(),
    });
    let no_bat = wait_until(&mut rx, |s| {
        s.selected_device_path.as_deref() == Some("/dev/hidraw1")
    });
    assert!(no_bat.battery_summary.is_none());
    assert!(no_bat.battery_percent.is_none());
    assert!(!no_bat.battery_is_low);
    assert!(no_bat.battery_warning.is_none());
    runtime.shutdown();
}

#[test]
fn end_to_end_post_flash_verification_verified_and_version_mismatch() {
    let image_bytes = make_official_firmware_image_bytes(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DEFAULT_PID,
        0x0004,
        0x0630,
    );

    // Case A: Verified post-flash firmware read confirms 0x0630
    let enum_ok = Arc::new(MockEnumerator::with_devices(vec![make_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "VER-1",
    )]));
    enum_ok.push_firmware_info("/dev/hidraw0", 0x0510);
    enum_ok.push_full_update_success("/dev/hidraw0", image_bytes.len());
    enum_ok.push_firmware_info("/dev/hidraw0", 0x0630);

    let rt_ok = AppRuntime::with_backends(
        ArcEnumerator(enum_ok),
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, image_bytes.clone()),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start rt_ok");

    let mut rx_ok = rt_ok.subscribe();
    rt_ok.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx_ok, |s| {
        !s.is_busy && s.can_continue_to_firmware && s.catalog_checked
    });
    rt_ok.dispatch(AppEvent::ContinueToFirmwareStep);
    rt_ok.dispatch(AppEvent::ContinueToConfirmStep);
    rt_ok.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx_ok, |s| s.can_start_update);
    rt_ok.dispatch(AppEvent::StartUpdate);

    let done_ok = wait_until(&mut rx_ok, |s| s.step == AppStep::Completed);
    assert!(done_ok.post_flash_verified);
    assert!(done_ok.is_up_to_date);
    assert_eq!(done_ok.version_comparison_text, "Up to date (0x0630)");
    assert_eq!(
        done_ok.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );
    rt_ok.shutdown();

    // Case B: Post-flash hardware still reports 0x0510 -> VersionMismatch warning on Completed
    let enum_mismatch = Arc::new(MockEnumerator::with_devices(vec![make_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "MIS-1",
    )]));
    enum_mismatch.push_firmware_info("/dev/hidraw0", 0x0510);
    enum_mismatch.push_full_update_success("/dev/hidraw0", image_bytes.len());
    enum_mismatch.push_firmware_info("/dev/hidraw0", 0x0510);

    let rt_mis = AppRuntime::with_backends(
        ArcEnumerator(enum_mismatch),
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, image_bytes),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start rt_mis");

    let mut rx_mis = rt_mis.subscribe();
    rt_mis.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx_mis, |s| {
        !s.is_busy && s.can_continue_to_firmware && s.catalog_checked
    });
    rt_mis.dispatch(AppEvent::ContinueToFirmwareStep);
    rt_mis.dispatch(AppEvent::ContinueToConfirmStep);
    rt_mis.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx_mis, |s| s.can_start_update);
    rt_mis.dispatch(AppEvent::StartUpdate);

    let done_mis = wait_until(&mut rx_mis, |s| s.step == AppStep::Completed);
    assert!(!done_mis.post_flash_verified);
    assert_eq!(
        done_mis.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Warning)
    );
    assert!(
        done_mis
            .completed_verification_note
            .as_deref()
            .is_some_and(|n| n.contains("0x0510") && n.contains("0x0630"))
    );
    rt_mis.shutdown();
}

#[test]
fn end_to_end_auto_inspect_cache_reuse_and_save_cached_bin() {
    let image_bytes = make_official_firmware_image_bytes(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES + 64,
        DEFAULT_PID,
        0x000B,
        0x0630,
    );
    let enum_mock = Arc::new(MockEnumerator::with_devices(vec![
        make_device("/dev/hidraw0", DEFAULT_PID, ""),
        make_device("/dev/hidraw1", DEFAULT_PID, ""),
    ]));
    enum_mock.push_firmware_info_with_hw("/dev/hidraw0", 0x0510, 0x000B, 0x0000_0811);
    enum_mock.set_serial_number("/dev/hidraw0", "F55301K0H14359961");
    enum_mock.set_battery_report("/dev/hidraw0", 8, BatteryChargingState::Charging);
    enum_mock.push_full_update_success("/dev/hidraw0", image_bytes.len());
    enum_mock.push_firmware_info_with_hw("/dev/hidraw0", 0x0630, 0x000B, 0x0000_0811);

    enum_mock.push_firmware_info_with_hw("/dev/hidraw1", 0x0520, 0x000B, 0x0000_0811);
    enum_mock.set_serial_number("/dev/hidraw1", "F55302K0H14359961");
    enum_mock.set_battery_report("/dev/hidraw1", 10, BatteryChargingState::Full);
    enum_mock.push_full_update_success("/dev/hidraw1", image_bytes.len());
    enum_mock.push_firmware_info_with_hw("/dev/hidraw1", 0x0630, 0x000B, 0x0000_0811);

    let download_calls = Arc::new(AtomicUsize::new(0));
    let http_mock = MockHttp::with_download_counter(
        SAMPLE_CATALOG_JSON,
        image_bytes.clone(),
        Arc::clone(&download_calls),
    );

    let runtime = AppRuntime::with_backends(
        ArcEnumerator(Arc::clone(&enum_mock)),
        http_mock,
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);

    // Both BDM-050 controllers are auto-inspected and catalog is auto-checked
    let ready = wait_until(&mut rx, |s| {
        !s.is_busy
            && s.catalog_checked
            && s.devices.len() == 2
            && s.devices.iter().all(|d| d.firmware_version.is_some())
    });
    assert_eq!(ready.devices[0].board_model, "BDM-050");
    assert_eq!(
        ready.devices[0].color_name.as_deref(),
        Some("Midnight Black")
    );
    assert_eq!(
        ready.devices[0].battery_summary.as_deref(),
        Some("80% (Charging)")
    );
    assert_eq!(ready.devices[1].board_model, "BDM-050");
    assert_eq!(ready.devices[1].color_name.as_deref(), Some("Cosmic Red"));
    assert_eq!(
        ready.devices[1].battery_summary.as_deref(),
        Some("100% (Full)")
    );
    assert_eq!(
        ready.version_comparison_text,
        "Update available: 0x0510 → 0x0630"
    );

    // Flash controller 1 (downloads once)
    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);
    runtime.dispatch(AppEvent::StartUpdate);

    let done1 = wait_until(&mut rx, |s| s.step == AppStep::Completed);
    assert!(done1.post_flash_verified);
    assert!(done1.has_cached_firmware);
    assert_eq!(
        done1.cached_firmware_suggested_name,
        "FWUPDATE000B_0630.bin"
    );
    assert!(done1.can_save_cached_firmware);
    assert_eq!(download_calls.load(Ordering::SeqCst), 1);

    // Save cached firmware binary to disk and verify contents
    let save_path = std::env::temp_dir().join(format!("ds_saved_{}.bin", std::process::id()));
    runtime.dispatch(AppEvent::SaveCachedFirmware {
        path: save_path.clone(),
    });
    let saved_snap = wait_until(&mut rx, |s| {
        s.banner
            .as_ref()
            .is_some_and(|b| b.title == "Firmware copy saved")
    });
    assert_eq!(
        saved_snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );
    assert_eq!(
        std::fs::read(&save_path).expect("read saved bin"),
        image_bytes
    );
    let _ = std::fs::remove_file(&save_path);

    // Reset, select controller 2 of the same target 000B, and flash -> reuses cache!
    runtime.dispatch(AppEvent::ResetFlow);
    runtime.dispatch(AppEvent::SelectDevice {
        path: "/dev/hidraw1".to_string(),
    });
    wait_until(&mut rx, |s| {
        s.step == AppStep::DetectController
            && s.selected_device_path.as_deref() == Some("/dev/hidraw1")
    });
    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);
    runtime.dispatch(AppEvent::StartUpdate);

    let done2 = wait_until(&mut rx, |s| s.step == AppStep::Completed);
    assert!(done2.post_flash_verified);
    assert_eq!(download_calls.load(Ordering::SeqCst), 1);
    runtime.shutdown();
}
