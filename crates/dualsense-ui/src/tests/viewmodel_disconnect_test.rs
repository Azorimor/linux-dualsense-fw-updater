// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use dualsense_core::{CoreError, DEFAULT_PID, DEFAULT_VID, UpdatePhase, UpdateProgress};

use super::{sample_bdm020_inspection, sample_catalog, sample_hid_device};
use crate::viewmodel::{
    AppCommand, AppEvent, AppState, AppStep, BannerSeverity, DeviceTarget, LocalFirmwareInfo,
    UpdateOutcome,
};

fn inspected_state_at_step(target_step: AppStep) -> AppState {
    let mut state = AppState::new();
    let d1 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
    state.update(AppEvent::DevicesScanned(Ok(vec![d1])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_bdm020_inspection(0x0520)),
    });
    if target_step == AppStep::DetectController {
        return state;
    }

    state.update(AppEvent::ContinueToFirmwareStep);
    if target_step == AppStep::SelectFirmware {
        return state;
    }

    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));
    state.update(AppEvent::ContinueToConfirmStep);
    if target_step == AppStep::ConfirmUpdate {
        return state;
    }

    state.update(AppEvent::SetWarningAcknowledged(true));
    state.update(AppEvent::StartUpdate);
    state
}

fn assert_disconnected_at_step0(state: &AppState) {
    let snap = state.snapshot();
    assert_eq!(snap.step, AppStep::DetectController);
    assert!(snap.selected_device_path.is_none());
    assert!(snap.selected_device.is_none());
    assert!(!snap.is_busy);
    assert!(!snap.can_continue_to_firmware);
    assert!(!snap.can_select_target);
    assert!(!snap.can_check_updates);
    assert!(!snap.can_select_file);
    assert!(!snap.can_review_update);
    assert!(!snap.can_start_update);
    assert!(!snap.warning_acknowledged);
    let banner = snap.banner.as_ref().expect("disconnect banner present");
    assert_eq!(banner.severity, BannerSeverity::Warning);
    assert_eq!(banner.title, "Controller disconnected");
    assert!(banner.text.contains("unplugged"));
}

#[test]
fn unplug_on_select_firmware_resets_to_step0_and_locks_progression() {
    let mut state = inspected_state_at_step(AppStep::SelectFirmware);
    let cmds = state.update(AppEvent::DevicesPollUpdated(Vec::new()));
    assert!(cmds.is_empty());
    assert_disconnected_at_step0(&state);
    assert!(state.snapshot().devices.is_empty());
}

#[test]
fn unplug_on_confirm_update_resets_to_step0_and_clears_acknowledgement() {
    let mut state = inspected_state_at_step(AppStep::ConfirmUpdate);
    state.update(AppEvent::SetWarningAcknowledged(true));
    assert!(state.snapshot().can_start_update);

    let cmds = state.update(AppEvent::DevicesPollUpdated(Vec::new()));
    assert!(cmds.is_empty());
    assert_disconnected_at_step0(&state);
}

#[test]
fn unplug_during_download_cancels_operation_and_ignores_late_cancel_result() {
    let mut state = inspected_state_at_step(AppStep::Updating);
    assert!(state.snapshot().can_cancel);

    let cmds = state.update(AppEvent::DevicesPollUpdated(Vec::new()));
    assert_eq!(cmds, vec![AppCommand::CancelActiveOperation]);
    assert_disconnected_at_step0(&state);

    // Late cancellation result from the aborted download must not jump to ConfirmUpdate.
    let late_cmds = state.update(AppEvent::UpdateFinished(Err(CoreError::Cancelled)));
    assert!(late_cmds.is_empty());
    assert_disconnected_at_step0(&state);
}

#[test]
fn unplug_during_catalog_check_cancels_and_ignores_late_cancel_result() {
    let mut state = inspected_state_at_step(AppStep::SelectFirmware);
    state.update(AppEvent::CheckForUpdates);
    assert!(state.snapshot().is_busy);

    let cmds = state.update(AppEvent::DevicesPollUpdated(Vec::new()));
    assert_eq!(cmds, vec![AppCommand::CancelActiveOperation]);
    assert_disconnected_at_step0(&state);

    let late_cmds = state.update(AppEvent::CatalogLoaded(Err(CoreError::Cancelled)));
    assert!(late_cmds.is_empty());
    assert_disconnected_at_step0(&state);
}

#[test]
fn unplug_on_detect_controller_clears_selection_and_replug_recovers_automatically() {
    let mut state = inspected_state_at_step(AppStep::DetectController);
    assert!(state.snapshot().can_continue_to_firmware);

    // Unplug on Step 0
    let cmds = state.update(AppEvent::DevicesPollUpdated(Vec::new()));
    assert!(cmds.is_empty());
    assert_disconnected_at_step0(&state);
    assert!(state.snapshot().board_model.is_empty());

    // Plug controller back in on Step 0 -> auto-selects and triggers ReadDeviceFirmwareInfo
    let d1 = sample_hid_device("/dev/hidraw1", DEFAULT_PID, "STD-1");
    let cmds = state.update(AppEvent::DevicesPollUpdated(vec![d1]));
    assert_eq!(
        cmds,
        vec![AppCommand::ReadDeviceFirmwareInfo {
            device: DeviceTarget {
                path: "/dev/hidraw1".to_string(),
                vendor_id: DEFAULT_VID,
                product_id: DEFAULT_PID,
            }
        }]
    );
    assert!(state.snapshot().is_busy);
    assert_eq!(
        state.snapshot().selected_device_path.as_deref(),
        Some("/dev/hidraw1")
    );

    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw1".to_string(),
        result: Ok(sample_bdm020_inspection(0x0520)),
    });
    let recovered = state.snapshot();
    assert!(!recovered.is_busy);
    assert!(recovered.can_continue_to_firmware);
    assert_eq!(
        recovered.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );
}

#[test]
fn unplug_on_completed_stays_on_completion_page_without_error() {
    let mut state = inspected_state_at_step(AppStep::Updating);
    state.update(AppEvent::UpdateProgressUpdated(UpdateProgress {
        phase: UpdatePhase::Finalizing,
        current_bytes: 65536,
        total_bytes: 65536,
        current_block: 2,
        total_blocks: 2,
        percentage: 100.0,
    }));
    state.update(AppEvent::UpdateFinished(Ok(UpdateOutcome::verified(
        0x0630,
    ))));
    assert_eq!(state.snapshot().step, AppStep::Completed);

    let cmds = state.update(AppEvent::DevicesPollUpdated(Vec::new()));
    assert!(cmds.is_empty());

    let snap = state.snapshot();
    assert_eq!(snap.step, AppStep::Completed);
    assert!(snap.error.is_none());
    assert_eq!(snap.completed_version.as_deref(), Some("0x0630"));
    assert_eq!(
        snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );
    assert_eq!(
        snap.banner.as_ref().map(|b| b.title.as_str()),
        Some("Firmware update complete")
    );
}

#[test]
fn flashing_ignores_poll_updates_and_preserves_inspected_devices_when_second_added() {
    let mut state = inspected_state_at_step(AppStep::DetectController);
    let d1 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
    let d2 = sample_hid_device("/dev/hidraw1", DEFAULT_PID, "STD-2");

    // Adding a second device keeps d1's inspected firmware version intact and queues background inspection for d2
    let cmds = state.update(AppEvent::DevicesPollUpdated(vec![d1.clone(), d2.clone()]));
    assert_eq!(
        cmds,
        vec![AppCommand::ReadDeviceFirmwareInfo {
            device: DeviceTarget {
                path: d2.path.clone(),
                vendor_id: d2.vendor_id,
                product_id: d2.product_id,
            }
        }]
    );
    let snap = state.snapshot();
    assert!(!snap.is_busy);
    assert_eq!(snap.devices.len(), 2);
    assert_eq!(
        snap.selected_device
            .as_ref()
            .and_then(|d| d.firmware_version),
        Some(0x0520)
    );

    // Move to Updating and transition to FlashingFirmware
    state.update(AppEvent::ContinueToFirmwareStep);
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));
    state.update(AppEvent::ContinueToConfirmStep);
    state.update(AppEvent::SetWarningAcknowledged(true));
    state.update(AppEvent::StartUpdate);
    state.update(AppEvent::UpdateProgressUpdated(UpdateProgress {
        phase: UpdatePhase::Writing,
        current_bytes: 1024,
        total_bytes: 2048,
        current_block: 1,
        total_blocks: 1,
        percentage: 50.0,
    }));
    assert!(state.is_flashing());

    let cmds = state.update(AppEvent::DevicesPollUpdated(Vec::new()));
    assert!(cmds.is_empty());
    assert_eq!(state.snapshot().step, AppStep::Updating);
}

#[test]
fn poll_devices_changed_ignores_serial_enrichment_and_detects_real_changes() {
    let mut state = AppState::new();
    let d1_no_serial = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "");
    assert!(state.poll_devices_changed(std::slice::from_ref(&d1_no_serial)));

    state.update(AppEvent::DevicesScanned(Ok(vec![d1_no_serial.clone()])));
    // Enrich serial via 0x81 report; hidraw list from OS still has empty serial
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(super::sample_inspection(
            0x0520,
            DEFAULT_PID,
            0x000B,
            0x0000_0811,
            Some("F55301K0H14359961"),
        )),
    });
    assert!(!state.poll_devices_changed(std::slice::from_ref(&d1_no_serial)));

    // Replug/disappear triggers change
    assert!(state.poll_devices_changed(&[]));
    let d2 = sample_hid_device("/dev/hidraw1", DEFAULT_PID, "");
    assert!(state.poll_devices_changed(&[d1_no_serial, d2]));
}

#[test]
fn step_transitions_are_guarded_when_no_connected_device_selected() {
    let mut state = AppState::new();
    assert!(state.update(AppEvent::ContinueToFirmwareStep).is_empty());
    assert_eq!(state.snapshot().step, AppStep::DetectController);

    assert!(state.update(AppEvent::CheckForUpdates).is_empty());
    assert!(state.update(AppEvent::ContinueToConfirmStep).is_empty());
    assert!(
        state
            .update(AppEvent::SelectLocalFile {
                path: PathBuf::from("/tmp/fw.bin"),
            })
            .is_empty()
    );
    assert!(state.update(AppEvent::StartUpdate).is_empty());

    // Even if local firmware was loaded earlier, unplugging blocks StartUpdate & ContinueToConfirmStep
    let mut state2 = inspected_state_at_step(AppStep::SelectFirmware);
    state2.update(AppEvent::LocalFileInspected {
        path: PathBuf::from("/tmp/fw.bin"),
        result: Ok(LocalFirmwareInfo::official(
            0x0630,
            262_144,
            DEFAULT_PID,
            "0004",
        )),
    });
    state2.update(AppEvent::DevicesPollUpdated(Vec::new()));
    assert_disconnected_at_step0(&state2);
    assert!(state2.update(AppEvent::ContinueToFirmwareStep).is_empty());
    assert_eq!(state2.snapshot().step, AppStep::DetectController);
}

#[test]
fn device_firmware_info_not_matched_clears_vanished_device() {
    let mut state = AppState::new();
    let d1 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
    state.update(AppEvent::DevicesScanned(Ok(vec![d1])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Err(CoreError::DevicePathNotMatched("/dev/hidraw0".to_string())),
    });

    let snap = state.snapshot();
    assert_eq!(snap.step, AppStep::Error);
    assert!(snap.devices.is_empty());
    assert!(snap.selected_device_path.is_none());
    assert_eq!(snap.error_title, "Controller disconnected");
}
