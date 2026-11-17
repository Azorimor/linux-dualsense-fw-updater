use std::path::PathBuf;

use dualsense_core::{CoreError, DEFAULT_PID, DownloadProgress, UpdatePhase, UpdateProgress};
use dualsense_ui::{AppEvent, AppState, LocalFirmwareInfo};

use super::{
    sample_bdm020_inspection, sample_catalog_for_0004, sample_hid_device, sample_inspection,
};
use crate::snapshot_adapter::{AdaptedSnapshot, BANNER_ERROR, BANNER_POSITIVE, BANNER_WARNING};

fn assert_adapted_disconnected_at_step0(adapted: &AdaptedSnapshot) {
    assert_eq!(adapted.step_index, 0);
    assert_eq!(adapted.step_title, "Connect Controller");
    assert_eq!(adapted.selected_device_index, -1);
    assert_eq!(adapted.selected_device_path, "");
    assert_eq!(adapted.selected_device_label, "");
    assert_eq!(adapted.board_model, "");
    assert_eq!(adapted.color_name, "");
    assert_eq!(adapted.serial_number, "");
    assert!(!adapted.board_supported);
    assert!(!adapted.is_busy);
    assert!(!adapted.progress_visible);
    assert!(!adapted.warning_acknowledged);
    assert!(adapted.has_banner);
    assert_eq!(adapted.banner_severity, BANNER_WARNING);
    assert_eq!(adapted.banner_title, "Controller disconnected");
    assert!(adapted.banner_text.contains("unplugged"));
    assert!(
        adapted
            .banner_display_text
            .contains("Controller disconnected")
    );
    assert!(adapted.can_scan);
    assert!(!adapted.can_continue_to_firmware);
    assert!(!adapted.can_select_target);
    assert!(!adapted.can_check_updates);
    assert!(!adapted.can_select_file);
    assert!(!adapted.can_review_update);
    assert!(!adapted.can_start_update);
    assert!(!adapted.can_cancel);
    assert!(!adapted.can_go_back);
}

#[test]
fn catalog_and_local_file_selection_populate_firmware_and_review_fields() {
    let mut state = AppState::new();
    state.update(AppEvent::DevicesScanned(Ok(vec![sample_hid_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "SN1",
    )])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_bdm020_inspection(0x0500)),
    });
    state.update(AppEvent::ContinueToFirmwareStep);
    state.update(AppEvent::CheckForUpdates);
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog_for_0004(0x0630))));

    let adapted = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(adapted.step_index, 1);
    assert!(adapted.catalog_checked);
    assert!(!adapted.use_local_file);
    assert!(adapted.can_select_target);
    assert_eq!(adapted.selected_target_latest_version, "0x0630");
    assert!(!adapted.selected_target_up_to_date);
    assert!(adapted.target_labels[0].contains("0x0630"));
    assert!(adapted.can_review_update);

    state.update(AppEvent::SelectLocalFile {
        path: PathBuf::from("/tmp/FWUPDATE0004.bin"),
    });
    state.update(AppEvent::LocalFileInspected {
        path: PathBuf::from("/tmp/FWUPDATE0004.bin"),
        result: Ok(LocalFirmwareInfo {
            version: 0x0630,
            size_bytes: 2 * 1024 * 1024,
        }),
    });
    state.update(AppEvent::ContinueToConfirmStep);
    state.update(AppEvent::SetWarningAcknowledged(true));

    let review = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(review.step_index, 2);
    assert!(review.use_local_file);
    assert!(!review.can_select_target);
    assert!(review.has_local_firmware);
    assert_eq!(review.local_firmware_name, "FWUPDATE0004.bin");
    assert_eq!(review.local_firmware_version, "0x0630");
    assert_eq!(review.local_firmware_size, "2.00 MB");
    assert!(review.has_update_summary);
    assert_eq!(review.summary_current_version, "0x0500");
    assert_eq!(review.summary_target_version, "0x0630");
    assert!(!review.summary_is_same_or_older);
    assert!(review.warning_acknowledged);
    assert!(review.can_start_update);
}

#[test]
fn progress_and_error_states_adapt_accurately() {
    let mut state = AppState::new();
    state.update(AppEvent::DevicesScanned(Ok(vec![sample_hid_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "SN1",
    )])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_inspection(
            "",
            "",
            0x0500,
            DEFAULT_PID,
            0x0004,
            0x0000_0411,
            None,
        )),
    });
    state.update(AppEvent::ContinueToFirmwareStep);
    state.update(AppEvent::CheckForUpdates);
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog_for_0004(0x0630))));
    state.update(AppEvent::ContinueToConfirmStep);
    state.update(AppEvent::SetWarningAcknowledged(true));
    state.update(AppEvent::StartUpdate);
    state.update(AppEvent::DownloadProgressUpdated(DownloadProgress {
        downloaded_bytes: 500,
        total_bytes: Some(1000),
    }));

    let downloading = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(downloading.step_index, 3);
    assert!(downloading.progress_visible);
    assert!(!downloading.progress_indeterminate);
    assert_eq!(downloading.progress_percentage, 50);
    assert!((downloading.progress_value - 0.5).abs() < f64::EPSILON);
    assert!(downloading.can_cancel);

    state.update(AppEvent::UpdateProgressUpdated(UpdateProgress {
        phase: UpdatePhase::Writing,
        current_bytes: 32768,
        total_bytes: 65536,
        current_block: 1,
        total_blocks: 2,
        percentage: 50.0,
    }));
    let flashing = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(flashing.banner_severity, BANNER_WARNING);
    assert!(!flashing.can_cancel);

    state.update(AppEvent::UpdateFinished(Err(CoreError::Cancelled)));
    let cancelled = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(cancelled.step_index, 2);

    state.update(AppEvent::SetWarningAcknowledged(true));
    state.update(AppEvent::StartUpdate);
    state.update(AppEvent::UpdateFinished(Err(CoreError::CatalogHttp(
        "timeout".to_string(),
    ))));
    let errored = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(errored.step_index, 5);
    assert!(errored.has_error);
    assert_eq!(errored.banner_severity, BANNER_ERROR);
    assert!(!errored.error_title.is_empty());
    assert!(!errored.recovery_hint.is_empty());

    state.update(AppEvent::DismissError);
    state.update(AppEvent::SetWarningAcknowledged(true));
    state.update(AppEvent::StartUpdate);
    state.update(AppEvent::UpdateFinished(Ok(0x0630)));
    let completed = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(completed.step_index, 4);
    assert_eq!(completed.completed_version, "0x0630");
    assert_eq!(completed.banner_severity, BANNER_POSITIVE);
    assert_eq!(completed.progress_percentage, 100);
    assert!((completed.progress_value - 1.0).abs() < f64::EPSILON);
}

#[test]
fn devices_poll_updated_unplug_on_step1_and_step2_adapts_step0_banner_and_gating() {
    let mut state = AppState::new();
    state.update(AppEvent::DevicesScanned(Ok(vec![sample_hid_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "",
    )])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_inspection(
            "Oct  1 2024",
            "12:34:56",
            0x0520,
            DEFAULT_PID,
            0x000B,
            0x0000_0811,
            Some("F55301K0H14359961"),
        )),
    });
    state.update(AppEvent::ContinueToFirmwareStep);
    assert_eq!(
        AdaptedSnapshot::from_ui_snapshot(&state.snapshot()).step_index,
        1
    );

    // Unplug on Step 1
    state.update(AppEvent::DevicesPollUpdated(Vec::new()));
    let disconnected_step1 = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_adapted_disconnected_at_step0(&disconnected_step1);
    assert!(!disconnected_step1.has_devices);
    assert_eq!(disconnected_step1.device_count, 0);

    // Reconnect and advance to Step 2 with acknowledged warning, then unplug
    state.update(AppEvent::DevicesPollUpdated(vec![sample_hid_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "",
    )]));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_bdm020_inspection(0x0500)),
    });
    state.update(AppEvent::ContinueToFirmwareStep);
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog_for_0004(0x0630))));
    state.update(AppEvent::ContinueToConfirmStep);
    state.update(AppEvent::SetWarningAcknowledged(true));
    assert!(AdaptedSnapshot::from_ui_snapshot(&state.snapshot()).can_start_update);

    state.update(AppEvent::DevicesPollUpdated(Vec::new()));
    let disconnected_step2 = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_adapted_disconnected_at_step0(&disconnected_step2);
    assert!(!disconnected_step2.has_devices);
}

#[test]
fn devices_poll_updated_unplug_during_download_and_replug_recovery_adapt_cleanly() {
    let mut state = AppState::new();
    state.update(AppEvent::DevicesScanned(Ok(vec![sample_hid_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "",
    )])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_bdm020_inspection(0x0500)),
    });
    state.update(AppEvent::ContinueToFirmwareStep);
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog_for_0004(0x0630))));
    state.update(AppEvent::ContinueToConfirmStep);
    state.update(AppEvent::SetWarningAcknowledged(true));
    state.update(AppEvent::StartUpdate);
    state.update(AppEvent::DownloadProgressUpdated(DownloadProgress {
        downloaded_bytes: 250,
        total_bytes: Some(1000),
    }));
    assert!(AdaptedSnapshot::from_ui_snapshot(&state.snapshot()).can_cancel);

    // Unplug during active download
    state.update(AppEvent::DevicesPollUpdated(Vec::new()));
    let disconnected = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_adapted_disconnected_at_step0(&disconnected);
    assert!(!disconnected.has_devices);

    // Replug a BDM-050 controller on Step 0
    state.update(AppEvent::DevicesPollUpdated(vec![sample_hid_device(
        "/dev/hidraw2",
        DEFAULT_PID,
        "",
    )]));
    let reading = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(reading.step_index, 0);
    assert!(reading.has_devices);
    assert!(reading.is_busy);
    assert!(reading.progress_visible);
    assert!(!reading.can_continue_to_firmware);

    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw2".to_string(),
        result: Ok(sample_inspection(
            "Oct  1 2024",
            "12:34:56",
            0x0520,
            DEFAULT_PID,
            0x000B,
            0x0000_0811,
            Some("F55301K0H14359961"),
        )),
    });
    let recovered = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(recovered.step_index, 0);
    assert!(recovered.has_devices);
    assert_eq!(recovered.selected_device_index, 0);
    assert_eq!(recovered.selected_device_path, "/dev/hidraw2");
    assert_eq!(recovered.board_model, "BDM-050");
    assert_eq!(recovered.color_name, "Midnight Black");
    assert_eq!(recovered.detected_target_id, "000B");
    assert_eq!(recovered.banner_severity, BANNER_POSITIVE);
    assert!(recovered.can_continue_to_firmware);
}

#[test]
fn devices_poll_updated_partial_unplug_keeps_remaining_device_unselected() {
    let mut state = AppState::new();
    let d0 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "SN-0");
    let d1 = sample_hid_device("/dev/hidraw1", DEFAULT_PID, "SN-1");
    state.update(AppEvent::DevicesScanned(Ok(vec![d0, d1.clone()])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_bdm020_inspection(0x0520)),
    });
    state.update(AppEvent::ContinueToFirmwareStep);

    // Unplug selected d0 while d1 stays connected
    state.update(AppEvent::DevicesPollUpdated(vec![d1]));
    let adapted = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_adapted_disconnected_at_step0(&adapted);
    assert!(adapted.has_devices);
    assert_eq!(adapted.device_count, 1);
    assert_eq!(adapted.device_paths, vec!["/dev/hidraw1"]);
}
