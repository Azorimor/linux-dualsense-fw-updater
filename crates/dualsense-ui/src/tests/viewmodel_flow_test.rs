// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use dualsense_core::{
    CoreError, DEFAULT_PID, DEFAULT_VID, DUALSENSE_EDGE_PID, DownloadProgress, FirmwareCatalog,
    KNOWN_TARGETS, StartUpdateError, UpdateFailure, UpdatePhase, UpdateProgress,
};

use super::{sample_bdm020_inspection, sample_catalog, sample_edge_inspection, sample_hid_device};
use crate::viewmodel::{
    AppCommand, AppEvent, AppState, AppStep, BannerSeverity, DeviceTarget, LocalFirmwareInfo,
    UpdateOutcome,
};

#[test]
fn initial_state_has_known_targets_and_proper_gating() {
    let state = AppState::new();
    let snap = state.snapshot();

    assert_eq!(snap.step, AppStep::DetectController);
    assert_eq!(snap.step_index, 0);
    assert_eq!(snap.targets.len(), KNOWN_TARGETS.len());
    assert_eq!(snap.selected_target_id, "0004");
    assert!(!snap.is_busy);
    assert!(snap.can_scan);
    assert!(!snap.can_continue_to_firmware);
    assert!(!snap.can_select_target);
    assert!(!snap.can_check_updates);
    assert!(!snap.can_select_file);
    assert!(!snap.can_review_update);
    assert!(!snap.can_start_update);
    assert!(!snap.can_cancel);
    assert!(!snap.can_go_back);
    assert!(!snap.board_supported);
    assert!(snap.unsupported_board_warning.is_none());
    assert_eq!(
        snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Info)
    );
}

#[test]
fn scan_select_and_auto_edge_target_selection_flow() {
    let mut state = AppState::new();
    let cmds = state.update(AppEvent::ScanDevices);
    assert_eq!(cmds, vec![AppCommand::ScanDevices]);
    let busy_snap = state.snapshot();
    assert!(busy_snap.is_busy);
    assert!(!busy_snap.can_scan);
    assert!(busy_snap.can_cancel);
    assert!(busy_snap.progress.indeterminate);

    // Empty scan shows helpful USB cable warning
    let empty_cmds = state.update(AppEvent::DevicesScanned(Ok(Vec::new())));
    assert!(empty_cmds.is_empty());
    let empty_snap = state.snapshot();
    assert!(!empty_snap.is_busy);
    assert!(!empty_snap.can_continue_to_firmware);
    assert_eq!(
        empty_snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Warning)
    );

    // Scan finding two devices (standard + Edge) auto-probes both USB devices
    let d1 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
    let d2 = sample_hid_device("/dev/hidraw1", DUALSENSE_EDGE_PID, "EDGE-1");
    let cmds = state.update(AppEvent::DevicesScanned(Ok(vec![d1.clone(), d2.clone()])));
    assert_eq!(
        cmds,
        vec![
            AppCommand::ReadDeviceFirmwareInfo {
                device: DeviceTarget {
                    path: "/dev/hidraw0".to_string(),
                    vendor_id: DEFAULT_VID,
                    product_id: DEFAULT_PID,
                }
            },
            AppCommand::ReadDeviceFirmwareInfo {
                device: DeviceTarget {
                    path: "/dev/hidraw1".to_string(),
                    vendor_id: DEFAULT_VID,
                    product_id: DUALSENSE_EDGE_PID,
                }
            },
        ]
    );

    // Firmware info arrives for first device -> triggers background catalog fetch without blocking
    let auto_cmds = state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_bdm020_inspection(0x0520)),
    });
    assert_eq!(auto_cmds, vec![AppCommand::FetchCatalog]);
    let snap = state.snapshot();
    assert!(snap.can_continue_to_firmware);
    assert!(snap.catalog_checking);
    assert!(snap.board_supported);
    assert!(snap.can_select_target);
    assert_eq!(snap.board_model, "BDM-020");
    assert_eq!(snap.detected_target_id.as_deref(), Some("0004"));
    assert_eq!(
        snap.selected_device
            .as_ref()
            .map(|d| d.formatted_firmware_version.as_str()),
        Some("0x0520")
    );
    assert_eq!(
        snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );

    // Selecting Edge controller auto-selects target 0044 (already queued, waits for info)
    let cmds = state.update(AppEvent::SelectDevice {
        path: "/dev/hidraw1".to_string(),
    });
    assert!(cmds.is_empty());
    assert!(state.snapshot().is_busy);
    assert_eq!(state.snapshot().selected_target_id, "0044");
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw1".to_string(),
        result: Ok(sample_edge_inspection(0x0217)),
    });

    // Selecting back to standard controller switches away from 0044 without re-probing
    let cmds = state.update(AppEvent::SelectDevice {
        path: "/dev/hidraw0".to_string(),
    });
    assert!(cmds.is_empty());
    assert_eq!(state.snapshot().selected_target_id, "0004");
}

#[test]
fn guided_catalog_confirm_progress_and_completion_flow() {
    let mut state = AppState::new();
    let d1 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
    state.update(AppEvent::DevicesScanned(Ok(vec![d1])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_bdm020_inspection(0x0520)),
    });
    state.update(AppEvent::ContinueToFirmwareStep);

    // Background catalog check started automatically; deliver it first, then verify manual Refresh
    assert!(state.snapshot().catalog_checking);
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));

    let snap = state.snapshot();
    assert_eq!(snap.step, AppStep::SelectFirmware);
    assert!(snap.can_check_updates);
    assert!(snap.can_select_file);
    assert!(snap.can_review_update);
    assert!(snap.can_go_back);

    // Manual CheckForUpdates / Refresh still works
    let cmds = state.update(AppEvent::CheckForUpdates);
    assert_eq!(cmds, vec![AppCommand::FetchCatalog]);
    assert!(state.snapshot().can_cancel);
    assert!(!state.snapshot().can_check_updates);

    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));
    let snap = state.snapshot();
    assert!(snap.catalog_checked);
    assert!(snap.can_review_update);
    assert_eq!(
        snap.selected_target.as_ref().and_then(|t| t.latest_version),
        Some(0x0630)
    );
    assert_eq!(
        snap.selected_target.as_ref().and_then(|t| t.is_up_to_date),
        Some(false)
    );

    // Continue to ConfirmUpdate
    state.update(AppEvent::ContinueToConfirmStep);
    let confirm_snap = state.snapshot();
    assert_eq!(confirm_snap.step, AppStep::ConfirmUpdate);
    assert!(!confirm_snap.can_start_update);
    assert!(
        confirm_snap
            .irreversible_warning
            .contains("cannot be downgraded")
    );
    let summary = confirm_snap.update_summary.expect("summary present");
    assert_eq!(summary.current_version_label, "0x0520");
    assert_eq!(summary.target_version_label, "0x0630");
    assert!(!summary.is_same_or_older_version);

    // Acknowledge warning and start update
    state.update(AppEvent::SetWarningAcknowledged(true));
    assert!(state.snapshot().can_start_update);

    let cmds = state.update(AppEvent::StartUpdate);
    assert_eq!(cmds.len(), 1);
    assert!(matches!(cmds[0], AppCommand::DownloadAndInstall { .. }));
    let dl_snap = state.snapshot();
    assert_eq!(dl_snap.step, AppStep::Updating);
    assert!(dl_snap.is_busy);
    assert!(dl_snap.can_cancel);
    assert!(!dl_snap.can_start_update);

    // Download progress formatting
    state.update(AppEvent::DownloadProgressUpdated(DownloadProgress {
        downloaded_bytes: 45,
        total_bytes: Some(100),
    }));
    let dl_prog = state.snapshot();
    assert_eq!(dl_prog.progress.status_text, "Downloading firmware… (45%)");
    assert_eq!(dl_prog.progress.percentage, Some(45));
    assert!(dl_prog.can_cancel);

    // Indeterminate download progress formatting
    state.update(AppEvent::DownloadProgressUpdated(DownloadProgress {
        downloaded_bytes: 2048,
        total_bytes: None,
    }));
    let dl_indet = state.snapshot();
    assert_eq!(dl_indet.progress.status_text, "Downloading firmware…");
    assert!(dl_indet.progress.indeterminate);
    assert_eq!(dl_indet.progress.detail_text, "2.0 KB downloaded");

    // Update transitions to flashing -> can_cancel becomes false!
    state.update(AppEvent::UpdateProgressUpdated(UpdateProgress {
        phase: UpdatePhase::Starting,
        current_bytes: 0,
        total_bytes: 65536,
        current_block: 0,
        total_blocks: 2,
        percentage: 0.0,
    }));
    assert!(!state.snapshot().can_cancel);
    assert_eq!(
        state.snapshot().progress.status_text,
        "Preparing controller for update…"
    );

    state.update(AppEvent::UpdateProgressUpdated(UpdateProgress {
        phase: UpdatePhase::Writing,
        current_bytes: 40632,
        total_bytes: 65536,
        current_block: 2,
        total_blocks: 2,
        percentage: 62.0,
    }));
    assert_eq!(
        state.snapshot().progress.status_text,
        "Installing firmware… (62%)"
    );
    assert_eq!(state.snapshot().progress.percentage, Some(62));

    state.update(AppEvent::UpdateProgressUpdated(UpdateProgress {
        phase: UpdatePhase::Verifying,
        current_bytes: 65536,
        total_bytes: 65536,
        current_block: 2,
        total_blocks: 2,
        percentage: 100.0,
    }));
    assert_eq!(
        state.snapshot().progress.status_text,
        "Verifying installed firmware…"
    );

    state.update(AppEvent::UpdateProgressUpdated(UpdateProgress {
        phase: UpdatePhase::Finalizing,
        current_bytes: 65536,
        total_bytes: 65536,
        current_block: 2,
        total_blocks: 2,
        percentage: 100.0,
    }));
    assert_eq!(
        state.snapshot().progress.status_text,
        "Finalizing firmware update…"
    );

    // Finish update
    state.update(AppEvent::UpdateFinished(Ok(UpdateOutcome::verified(
        0x0630,
    ))));
    let done_snap = state.snapshot();
    assert_eq!(done_snap.step, AppStep::Completed);
    assert!(!done_snap.is_busy);
    assert_eq!(done_snap.completed_version.as_deref(), Some("0x0630"));
    assert_eq!(done_snap.progress.percentage, Some(100));
    assert_eq!(
        done_snap.selected_target.and_then(|t| t.is_up_to_date),
        Some(true)
    );
    assert_eq!(
        done_snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );

    // ResetFlow returns cleanly to DetectController
    state.update(AppEvent::ResetFlow);
    assert_eq!(state.snapshot().step, AppStep::DetectController);
}

#[test]
fn local_file_flow_cancellation_and_friendly_error_recovery() {
    let mut state = AppState::new();
    let d1 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
    state.update(AppEvent::DevicesScanned(Ok(vec![d1])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_bdm020_inspection(0x0630)),
    });
    state.update(AppEvent::ContinueToFirmwareStep);

    let path = PathBuf::from("/tmp/fw.bin");
    let cmds = state.update(AppEvent::SelectLocalFile { path: path.clone() });
    assert_eq!(
        cmds,
        vec![AppCommand::InspectLocalFile { path: path.clone() }]
    );

    state.update(AppEvent::LocalFileInspected {
        path: path.clone(),
        result: Ok(LocalFirmwareInfo::official(
            0x0630,
            1_572_864,
            DEFAULT_PID,
            "0004",
        )),
    });
    let snap = state.snapshot();
    assert!(snap.can_review_update);
    assert_eq!(
        snap.local_firmware.as_ref().map(|f| f.size_label.as_str()),
        Some("1.50 MB")
    );

    // Confirm & detect same-or-older version warning flag
    state.update(AppEvent::ContinueToConfirmStep);
    assert!(
        state
            .snapshot()
            .update_summary
            .expect("summary")
            .is_same_or_older_version
    );

    // GoBack and back forward
    state.update(AppEvent::GoBack);
    assert_eq!(state.snapshot().step, AppStep::SelectFirmware);
    state.update(AppEvent::ContinueToConfirmStep);
    state.update(AppEvent::SetWarningAcknowledged(true));

    let cmds = state.update(AppEvent::StartUpdate);
    assert!(matches!(cmds[0], AppCommand::InstallLocalFirmware { .. }));
    // Local file flashing cannot be cancelled once started
    assert!(!state.snapshot().can_cancel);

    // Simulate failure from controller
    state.update(AppEvent::UpdateFinished(Err(CoreError::UpdateFailed(
        UpdateFailure::StartUpdate(StartUpdateError::HeaderVersionCheckError),
    ))));
    let err_snap = state.snapshot();
    assert_eq!(err_snap.step, AppStep::Error);
    assert!(err_snap.error_title.contains("already has this"));
    assert!(!err_snap.recovery_hint.is_empty());

    // DismissError returns to ConfirmUpdate with warning unchecked
    state.update(AppEvent::DismissError);
    let recovered = state.snapshot();
    assert_eq!(recovered.step, AppStep::ConfirmUpdate);
    assert!(!recovered.warning_acknowledged);
    assert!(recovered.error.is_none());
}

#[test]
fn download_cancellation_returns_to_confirm_step_cleanly() {
    let mut state = AppState::new();
    let d1 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
    state.update(AppEvent::DevicesScanned(Ok(vec![d1])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_bdm020_inspection(0x0520)),
    });
    state.update(AppEvent::ContinueToFirmwareStep);
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));
    state.update(AppEvent::ContinueToConfirmStep);
    state.update(AppEvent::SetWarningAcknowledged(true));
    state.update(AppEvent::StartUpdate);

    let cancel_cmds = state.update(AppEvent::RequestCancel);
    assert_eq!(cancel_cmds, vec![AppCommand::CancelActiveOperation]);

    state.update(AppEvent::UpdateFinished(Err(CoreError::Cancelled)));
    let snap = state.snapshot();
    assert_eq!(snap.step, AppStep::ConfirmUpdate);
    assert!(!snap.warning_acknowledged);
    assert_eq!(
        snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Info)
    );
    assert!(
        snap.banner
            .as_ref()
            .expect("banner")
            .text
            .contains("before anything was written")
    );
}

#[test]
fn handles_extra_catalog_target_and_busy_guards() {
    let mut state = AppState::new();
    // Select non-existent target does nothing
    state.update(AppEvent::SelectTarget {
        target_id: "9999".to_string(),
    });
    assert_eq!(state.snapshot().selected_target_id, "0004");

    // Extra target in catalog is appended cleanly
    let catalog = FirmwareCatalog::from_json(
        r#"{"FwUpdate0004LatestVersion":"0x0630","FwUpdate0099LatestVersion":"0x0100"}"#,
    )
    .expect("catalog");
    state.update(AppEvent::CatalogLoaded(Ok(catalog)));
    assert!(
        state
            .snapshot()
            .targets
            .iter()
            .any(|t| t.target_id == "0099" && t.latest_version == Some(0x0100))
    );

    // Busy guard ignores concurrent user commands except RequestCancel
    state.update(AppEvent::ScanDevices);
    assert!(state.update(AppEvent::ScanDevices).is_empty());
    assert!(state.update(AppEvent::GoBack).is_empty());
    assert!(state.update(AppEvent::ResetFlow).is_empty());
    assert!(state.update(AppEvent::DismissError).is_empty());
    assert!(state.update(AppEvent::StartUpdate).is_empty());
}
