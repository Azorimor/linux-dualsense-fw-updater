// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use common::{
    ArcEnumerator, MockEnumerator, MockHttp, NoopSleeper, SAMPLE_CATALOG_JSON, fast_update_config,
    make_device, make_firmware_image_bytes, wait_until,
};
use dualsense_core::{DEFAULT_PID, REPORT_ID_UPDATE_COMMAND, UpdateCommand, WRITE_BLOCK_SIZE};
use dualsense_ui::{AppEvent, AppRuntime, AppStep, BannerSeverity, ProgressStage};

#[test]
fn live_unplug_on_step1_and_step2_resets_to_detect_and_locks_progression() {
    let enum_mock = Arc::new(MockEnumerator::with_devices(vec![make_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "STD-01",
    )]));
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);

    let runtime = AppRuntime::with_backends_and_poll_interval(
        ArcEnumerator(Arc::clone(&enum_mock)),
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        Some(Duration::from_millis(20)),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);

    // Move to Step 1 (SelectFirmware) and unplug controller
    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    wait_until(&mut rx, |s| s.step == AppStep::SelectFirmware);
    enum_mock.set_list_result(Ok(Vec::new()));

    let after_unplug_step1 = wait_until(&mut rx, |s| {
        s.step == AppStep::DetectController
            && s.devices.is_empty()
            && s.banner.as_ref().map(|b| b.title.as_str()) == Some("Controller disconnected")
    });
    assert!(!after_unplug_step1.can_continue_to_firmware);
    assert!(!after_unplug_step1.can_select_target);
    assert!(!after_unplug_step1.can_check_updates);
    assert!(!after_unplug_step1.can_review_update);
    assert!(!after_unplug_step1.can_start_update);
    assert!(after_unplug_step1.selected_device.is_none());

    // Replug controller -> poller recovers automatically on Step 0
    enum_mock.push_firmware_info("/dev/hidraw1", 0x0520);
    enum_mock.set_list_result(Ok(vec![make_device("/dev/hidraw1", DEFAULT_PID, "STD-01")]));
    wait_until(&mut rx, |s| {
        !s.is_busy
            && s.step == AppStep::DetectController
            && s.can_continue_to_firmware
            && s.selected_device_path.as_deref() == Some("/dev/hidraw1")
    });

    // Advance to Step 2 (ConfirmUpdate), acknowledge warning, then unplug again
    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::CheckForUpdates);
    wait_until(&mut rx, |s| !s.is_busy && s.can_review_update);
    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    enum_mock.set_list_result(Ok(Vec::new()));
    let after_unplug_step2 = wait_until(&mut rx, |s| {
        s.step == AppStep::DetectController
            && s.devices.is_empty()
            && s.banner.as_ref().map(|b| b.title.as_str()) == Some("Controller disconnected")
    });
    assert!(!after_unplug_step2.warning_acknowledged);
    assert!(!after_unplug_step2.can_start_update);
    assert!(!after_unplug_step2.can_continue_to_firmware);
    runtime.shutdown();
}

#[test]
fn live_unplug_during_download_cancels_download_and_stays_at_step0() {
    let enum_mock = Arc::new(MockEnumerator::with_devices(vec![make_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "STD-01",
    )]));
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);

    let runtime = AppRuntime::with_backends_and_poll_interval(
        ArcEnumerator(Arc::clone(&enum_mock)),
        MockHttp::slow_cancellable(SAMPLE_CATALOG_JSON),
        NoopSleeper,
        fast_update_config(),
        Some(Duration::from_millis(20)),
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
    wait_until(&mut rx, |s| {
        s.step == AppStep::Updating
            && s.progress.stage == ProgressStage::Downloading
            && s.can_cancel
    });

    // Unplug during active download
    enum_mock.set_list_result(Ok(Vec::new()));
    let disconnected = wait_until(&mut rx, |s| {
        s.step == AppStep::DetectController
            && !s.is_busy
            && s.banner.as_ref().map(|b| b.title.as_str()) == Some("Controller disconnected")
    });
    assert!(!disconnected.can_continue_to_firmware);
    assert!(disconnected.selected_device.is_none());

    // Give the aborted download task time to settle and confirm we stay on Step 0.
    thread::sleep(Duration::from_millis(60));
    let settled = runtime.snapshot();
    assert_eq!(settled.step, AppStep::DetectController);
    assert_eq!(
        settled.banner.as_ref().map(|b| b.title.as_str()),
        Some("Controller disconnected")
    );
    assert!(
        !enum_mock
            .sent_reports("/dev/hidraw0")
            .iter()
            .any(|r| r[0] == REPORT_ID_UPDATE_COMMAND && r[1] == UpdateCommand::StartUpdate as u8)
    );
    runtime.shutdown();
}

#[test]
fn live_replug_from_empty_initial_state_auto_detects_and_reads_board() {
    let enum_mock = Arc::new(MockEnumerator::default());

    let runtime = AppRuntime::with_backends_and_poll_interval(
        ArcEnumerator(Arc::clone(&enum_mock)),
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        Some(Duration::from_millis(20)),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    assert!(!runtime.snapshot().can_continue_to_firmware);

    // Plug in a BDM-050 controller without clicking Scan
    enum_mock.push_firmware_info_with_hw("/dev/hidraw0", 0x0520, 0x000B, 0x0000_0811);
    enum_mock.set_serial_number("/dev/hidraw0", "F55301K0H14359961");
    enum_mock.set_list_result(Ok(vec![make_device("/dev/hidraw0", DEFAULT_PID, "")]));

    let recovered = wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);
    assert_eq!(recovered.board_model, "BDM-050");
    assert_eq!(recovered.detected_target_id.as_deref(), Some("000B"));
    assert_eq!(recovered.color_name.as_deref(), Some("Midnight Black"));
    assert_eq!(
        recovered.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );
    runtime.shutdown();
}

#[test]
fn live_unplug_on_completed_leaves_completion_page_undisturbed() {
    let image_bytes = make_firmware_image_bytes(WRITE_BLOCK_SIZE + 64, 0x0630);
    let enum_mock = Arc::new(MockEnumerator::with_devices(vec![make_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "STD-01",
    )]));
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);
    enum_mock.push_full_update_success("/dev/hidraw0", image_bytes.len());

    let runtime = AppRuntime::with_backends_and_poll_interval(
        ArcEnumerator(Arc::clone(&enum_mock)),
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, image_bytes),
        NoopSleeper,
        fast_update_config(),
        Some(Duration::from_millis(20)),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);
    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::CheckForUpdates);
    wait_until(&mut rx, |s| !s.is_busy && s.can_review_update);
    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);
    runtime.dispatch(AppEvent::StartUpdate);
    wait_until(&mut rx, |s| s.step == AppStep::Completed);

    // Unplug controller after update finishes -> devices updates silently, Completed stays intact
    enum_mock.set_list_result(Ok(Vec::new()));
    let after_unplug = wait_until(&mut rx, |s| s.devices.is_empty());
    assert_eq!(after_unplug.step, AppStep::Completed);
    assert!(after_unplug.error.is_none());
    assert_eq!(after_unplug.completed_version.as_deref(), Some("0x0630"));
    assert_eq!(
        after_unplug.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );
    assert_eq!(
        after_unplug.banner.as_ref().map(|b| b.title.as_str()),
        Some("Firmware update complete")
    );
    runtime.shutdown();
}

#[test]
fn polling_skips_flashing_and_avoids_redundant_snapshots_when_unchanged() {
    let image_bytes = make_firmware_image_bytes(512, 0x0630);
    let temp_path = std::env::temp_dir().join(format!(
        "dualsense_ui_poll_flash_{}.bin",
        std::process::id()
    ));
    std::fs::write(&temp_path, &image_bytes).expect("write temp bin");

    let enum_mock = Arc::new(MockEnumerator::with_devices(vec![make_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "STD-01",
    )]));
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);
    enum_mock.push_full_update_success("/dev/hidraw0", image_bytes.len());

    let snapshot_count = Arc::new(AtomicUsize::new(0));
    let snapshot_count_cb = Arc::clone(&snapshot_count);

    let runtime = AppRuntime::with_backends_and_poll_interval(
        ArcEnumerator(Arc::clone(&enum_mock)),
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        Some(Duration::from_millis(15)),
        move |_| {
            snapshot_count_cb.fetch_add(1, Ordering::SeqCst);
        },
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);

    // Wait several poll intervals with unchanged device list -> snapshot count must not grow.
    let count_before_idle = snapshot_count.load(Ordering::SeqCst);
    thread::sleep(Duration::from_millis(80));
    assert_eq!(snapshot_count.load(Ordering::SeqCst), count_before_idle);
    assert!(enum_mock.list_call_count() >= 3);

    // Advance to ConfirmUpdate with LocalFile (which enters FlashingFirmware immediately).
    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::SelectLocalFile {
        path: temp_path.clone(),
    });
    wait_until(&mut rx, |s| !s.is_busy && s.can_review_update);
    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    // Slow down HID responses so flashing lasts ~120ms across several 15ms poll ticks.
    enum_mock.set_report_delay(Some(Duration::from_millis(30)));
    enum_mock.reset_list_call_count();
    runtime.dispatch(AppEvent::StartUpdate);
    wait_until(&mut rx, |s| s.step == AppStep::Completed);

    // During FlashingFirmware, list_devices must not be polled at all.
    assert_eq!(enum_mock.list_call_count(), 0);

    let _ = std::fs::remove_file(&temp_path);
    runtime.shutdown();
}
