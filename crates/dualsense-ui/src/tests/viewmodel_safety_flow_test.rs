// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use dualsense_core::{
    BatteryChargingState, BatteryStatus, CoreError, DEFAULT_PID, DUALSENSE_EDGE_PID, FirmwareInfo,
    LocalImageCompatibility, PostFlashVerification, PostFlashVerificationStatus,
    REPORT_ID_FIRMWARE_INFO,
};

use super::{
    sample_bdm020_inspection, sample_bluetooth_hid_device, sample_catalog, sample_hid_device,
    sample_inspection_with_battery,
};
use crate::viewmodel::{
    AppCommand, AppEvent, AppState, AppStep, BannerSeverity, LocalFirmwareInfo, UpdateOutcome,
};

fn make_verified_info(version: u16, build_date: &str, build_time: &str) -> FirmwareInfo {
    let mut raw = vec![0u8; 64];
    raw[0] = REPORT_ID_FIRMWARE_INFO;
    raw[1..1 + build_date.len()].copy_from_slice(build_date.as_bytes());
    raw[12..12 + build_time.len()].copy_from_slice(build_time.as_bytes());
    raw[22..24].copy_from_slice(&0x0004u16.to_le_bytes());
    raw[24..28].copy_from_slice(&0x0000_0411u32.to_le_bytes());
    raw[44..46].copy_from_slice(&version.to_le_bytes());
    FirmwareInfo::from_report(raw).expect("valid firmware info")
}

#[test]
fn bluetooth_only_detection_and_mixed_bus_handling() {
    let mut state = AppState::new();
    let bt1 = sample_bluetooth_hid_device("/dev/hidraw9", DEFAULT_PID, "BT-1");
    let bt2 = sample_bluetooth_hid_device("/dev/hidraw10", DUALSENSE_EDGE_PID, "BT-2");

    // Single Bluetooth device shows tailored USB-cable guidance
    let cmds = state.update(AppEvent::DevicesScanned(Ok(vec![bt1.clone()])));
    assert!(cmds.is_empty());
    let snap = state.snapshot();
    assert!(snap.bluetooth_only_detected);
    assert_eq!(snap.bluetooth_device_count, 1);
    assert!(snap.devices.is_empty());
    assert!(!snap.can_continue_to_firmware);
    let banner = snap.banner.expect("bluetooth banner");
    assert_eq!(banner.severity, BannerSeverity::Warning);
    assert!(banner.title.contains("Bluetooth"));
    assert!(banner.text.contains("USB data cable"));

    // Poll with unchanged BT device does not trigger poll_devices_changed
    assert!(!state.poll_devices_changed(std::slice::from_ref(&bt1)));
    assert!(state.poll_devices_changed(&[bt1.clone(), bt2.clone()]));

    // Multiple BT devices update count cleanly
    state.update(AppEvent::DevicesPollUpdated(vec![bt1.clone(), bt2]));
    assert!(state.snapshot().bluetooth_only_detected);
    assert_eq!(state.snapshot().bluetooth_device_count, 2);

    // Plugging a USB device alongside BT shows USB device and clears bluetooth_only_detected
    let usb = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "USB-1");
    let cmds = state.update(AppEvent::DevicesPollUpdated(vec![bt1, usb]));
    assert_eq!(cmds.len(), 1);
    let mixed = state.snapshot();
    assert!(!mixed.bluetooth_only_detected);
    assert_eq!(mixed.bluetooth_device_count, 1);
    assert_eq!(mixed.devices.len(), 1);
    assert_eq!(mixed.devices[0].path, "/dev/hidraw0");
}

#[test]
fn local_bin_validation_blocks_incompatible_and_reevaluates_on_device_switch() {
    let mut state = AppState::new();
    let d_020 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-020");
    let d_050 = sample_hid_device("/dev/hidraw1", DEFAULT_PID, "STD-050");
    state.update(AppEvent::DevicesScanned(Ok(vec![d_020, d_050])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_bdm020_inspection(0x0510)),
    });
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw1".to_string(),
        result: Ok(sample_inspection_with_battery(
            0x0520,
            DEFAULT_PID,
            0x000B,
            0x0000_0811,
            None,
            None,
        )),
    });
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));
    state.update(AppEvent::ContinueToFirmwareStep);

    // 1) Missing Sony header -> Incompatible, blocks review/confirm
    let bad_hdr_path = PathBuf::from("/tmp/missing_header.bin");
    state.update(AppEvent::SelectLocalFile {
        path: bad_hdr_path.clone(),
    });
    state.update(AppEvent::LocalFileInspected {
        path: bad_hdr_path,
        result: Ok(LocalFirmwareInfo::invalid_header(
            0x0630,
            1024,
            "Firmware image is 1024 bytes, below the 262144 byte minimum for official Sony images.",
        )),
    });
    let bad_hdr = state.snapshot();
    assert_eq!(
        bad_hdr.local_firmware_compatibility,
        Some(LocalImageCompatibility::Incompatible)
    );
    assert!(!bad_hdr.local_firmware_compatible);
    assert!(bad_hdr.local_firmware_error.is_some());
    assert!(!bad_hdr.can_review_update);
    assert!(state.update(AppEvent::ContinueToConfirmStep).is_empty());
    assert_eq!(state.snapshot().step, AppStep::SelectFirmware);

    // 2) Wrong embedded PID (Edge PID on standard DualSense) -> Incompatible
    state.update(AppEvent::LocalFileInspected {
        path: PathBuf::from("/tmp/edge.bin"),
        result: Ok(LocalFirmwareInfo::official(
            0x0217,
            300_000,
            DUALSENSE_EDGE_PID,
            "0044",
        )),
    });
    let wrong_pid = state.snapshot();
    assert!(!wrong_pid.local_firmware_compatible);
    assert!(!wrong_pid.can_review_update);
    assert!(
        wrong_pid
            .local_firmware_error
            .as_deref()
            .is_some_and(|e| e.contains("PID"))
    );

    // 3) Valid 0004 image for BDM-020 -> Compatible, allows review
    state.update(AppEvent::LocalFileInspected {
        path: PathBuf::from("/tmp/fw_0004.bin"),
        result: Ok(LocalFirmwareInfo::official(
            0x0630,
            300_000,
            DEFAULT_PID,
            "0004",
        )),
    });
    let good = state.snapshot();
    assert_eq!(
        good.local_firmware_compatibility,
        Some(LocalImageCompatibility::Compatible)
    );
    assert!(good.local_firmware_compatible);
    assert!(good.local_firmware_error.is_none());
    assert!(good.local_firmware_warning.is_none());
    assert!(good.can_review_update);

    // 4) Switch to BDM-050 -> re-evaluates to Incompatible automatically!
    state.update(AppEvent::GoBack);
    state.update(AppEvent::SelectDevice {
        path: "/dev/hidraw1".to_string(),
    });
    state.update(AppEvent::ContinueToFirmwareStep);
    let after_switch = state.snapshot();
    assert_eq!(
        after_switch.local_firmware_compatibility,
        Some(LocalImageCompatibility::Incompatible)
    );
    assert!(!after_switch.local_firmware_compatible);
    assert!(!after_switch.can_review_update);
    assert!(
        after_switch
            .local_firmware_error
            .as_deref()
            .is_some_and(|e| e.contains("0004") && e.contains("000B"))
    );
}

#[test]
fn battery_status_warnings_and_charging_suppression() {
    let mut state = AppState::new();
    let d_crit = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "CRIT");
    let d_low = sample_hid_device("/dev/hidraw1", DEFAULT_PID, "LOW");
    let d_chg = sample_hid_device("/dev/hidraw2", DEFAULT_PID, "CHG");

    state.update(AppEvent::DevicesScanned(Ok(vec![d_crit, d_low, d_chg])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_inspection_with_battery(
            0x0510,
            DEFAULT_PID,
            0x0004,
            0x0000_0411,
            None,
            Some(BatteryStatus {
                level: 1,
                percentage: 10,
                charging_state: BatteryChargingState::Discharging,
            }),
        )),
    });
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw1".to_string(),
        result: Ok(sample_inspection_with_battery(
            0x0510,
            DEFAULT_PID,
            0x0004,
            0x0000_0411,
            None,
            Some(BatteryStatus {
                level: 2,
                percentage: 20,
                charging_state: BatteryChargingState::Discharging,
            }),
        )),
    });
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw2".to_string(),
        result: Ok(sample_inspection_with_battery(
            0x0510,
            DEFAULT_PID,
            0x0004,
            0x0000_0411,
            None,
            Some(BatteryStatus {
                level: 1,
                percentage: 10,
                charging_state: BatteryChargingState::Charging,
            }),
        )),
    });
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));

    // Critically low discharging battery
    let crit_snap = state.snapshot();
    assert_eq!(crit_snap.battery_percent, Some(10));
    assert_eq!(
        crit_snap.battery_summary.as_deref(),
        Some("10% (Discharging)")
    );
    assert!(crit_snap.battery_is_low);
    assert!(crit_snap.battery_is_critically_low);
    assert!(
        crit_snap
            .battery_warning
            .as_deref()
            .is_some_and(|w| w.contains("critically low"))
    );

    // Entering ConfirmUpdate surfaces battery warning banner
    state.update(AppEvent::ContinueToFirmwareStep);
    state.update(AppEvent::ContinueToConfirmStep);
    let confirm_snap = state.snapshot();
    assert_eq!(confirm_snap.step, AppStep::ConfirmUpdate);
    assert_eq!(
        confirm_snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Warning)
    );
    assert_eq!(
        confirm_snap.banner.as_ref().map(|b| b.title.as_str()),
        Some("Low controller battery")
    );

    // Low (20%) discharging battery
    state.update(AppEvent::GoBack);
    state.update(AppEvent::GoBack);
    state.update(AppEvent::SelectDevice {
        path: "/dev/hidraw1".to_string(),
    });
    let low_snap = state.snapshot();
    assert!(low_snap.battery_is_low);
    assert!(!low_snap.battery_is_critically_low);
    assert!(low_snap.battery_warning.is_some());

    // Low percentage (10%) while Charging -> no low battery warning
    state.update(AppEvent::SelectDevice {
        path: "/dev/hidraw2".to_string(),
    });
    let chg_snap = state.snapshot();
    assert_eq!(chg_snap.battery_percent, Some(10));
    assert!(!chg_snap.battery_is_low);
    assert!(!chg_snap.battery_is_critically_low);
    assert!(chg_snap.battery_warning.is_none());
}

#[test]
fn post_flash_verification_verified_unreachable_and_mismatch_states() {
    let setup_updating = || {
        let mut state = AppState::new();
        let d = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
        state.update(AppEvent::DevicesScanned(Ok(vec![d])));
        state.update(AppEvent::DeviceFirmwareInfoLoaded {
            path: "/dev/hidraw0".to_string(),
            result: Ok(sample_bdm020_inspection(0x0510)),
        });
        state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));
        state.update(AppEvent::ContinueToFirmwareStep);
        state.update(AppEvent::ContinueToConfirmStep);
        state.update(AppEvent::SetWarningAcknowledged(true));
        state.update(AppEvent::StartUpdate);
        state
    };

    // 1) Verified updates firmware_version, build_date, and build_time from hardware
    let mut verified_state = setup_updating();
    let hw_info = make_verified_info(0x0630, "Feb 12 2026", "09:15:00");
    verified_state.update(AppEvent::UpdateFinished(Ok(UpdateOutcome::new(
        0x0630,
        PostFlashVerification {
            status: PostFlashVerificationStatus::Verified,
            expected_version: 0x0630,
            firmware_info: Some(hw_info),
            attempts: 2,
            last_error: None,
        },
    ))));
    let v_snap = verified_state.snapshot();
    assert_eq!(v_snap.step, AppStep::Completed);
    assert!(v_snap.post_flash_verified);
    assert!(v_snap.completed_verification_note.is_some());
    let v_dev = v_snap.selected_device.expect("selected device");
    assert_eq!(v_dev.firmware_version, Some(0x0630));
    assert_eq!(v_dev.build_date.as_deref(), Some("Feb 12 2026"));
    assert_eq!(v_dev.build_time.as_deref(), Some("09:15:00"));
    assert_eq!(
        v_snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );

    // 2) Unreachable succeeds gracefully noting controller is rebooting
    let mut unreachable_state = setup_updating();
    unreachable_state.update(AppEvent::UpdateFinished(Ok(UpdateOutcome::new(
        0x0630,
        PostFlashVerification {
            status: PostFlashVerificationStatus::Unreachable,
            expected_version: 0x0630,
            firmware_info: None,
            attempts: 8,
            last_error: Some("re-enumerating".to_string()),
        },
    ))));
    let u_snap = unreachable_state.snapshot();
    assert_eq!(u_snap.step, AppStep::Completed);
    assert!(!u_snap.post_flash_verified);
    assert!(
        u_snap
            .completed_verification_note
            .as_deref()
            .is_some_and(|n| n.contains("rebooting"))
    );
    assert_eq!(
        u_snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );

    // 3) VersionMismatch surfaces clear warning on Completed
    let mut mismatch_state = setup_updating();
    let old_hw = make_verified_info(0x0510, "Oct  1 2024", "12:34:56");
    mismatch_state.update(AppEvent::UpdateFinished(Ok(UpdateOutcome::new(
        0x0630,
        PostFlashVerification {
            status: PostFlashVerificationStatus::VersionMismatch,
            expected_version: 0x0630,
            firmware_info: Some(old_hw),
            attempts: 1,
            last_error: None,
        },
    ))));
    let m_snap = mismatch_state.snapshot();
    assert_eq!(m_snap.step, AppStep::Completed);
    assert!(!m_snap.post_flash_verified);
    assert_eq!(
        m_snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Warning)
    );
    assert!(
        m_snap
            .completed_verification_note
            .as_deref()
            .is_some_and(|n| n.contains("0x0510") && n.contains("0x0630"))
    );
}

#[test]
fn background_catalog_check_offline_graceful_and_version_comparison_text() {
    let mut state = AppState::new();
    let d = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
    state.update(AppEvent::DevicesScanned(Ok(vec![d])));
    let cmds = state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_bdm020_inspection(0x0510)),
    });
    assert_eq!(cmds, vec![AppCommand::FetchCatalog]);

    // Step 0 Continue stays usable while background catalog check runs
    let checking = state.snapshot();
    assert!(checking.catalog_checking);
    assert!(!checking.is_busy);
    assert!(checking.can_continue_to_firmware);
    assert!(checking.version_comparison_text.contains("Checking"));

    // Background offline failure must NOT jump to AppStep::Error
    state.update(AppEvent::CatalogLoaded(Err(CoreError::CatalogHttp(
        "offline".to_string(),
    ))));
    let offline = state.snapshot();
    assert_eq!(offline.step, AppStep::DetectController);
    assert!(!offline.catalog_checking);
    assert!(!offline.catalog_checked);
    assert!(offline.error.is_none());

    // Navigate to Step 1 and run manual CheckForUpdates successfully
    state.update(AppEvent::ContinueToFirmwareStep);
    let manual_cmds = state.update(AppEvent::CheckForUpdates);
    assert_eq!(manual_cmds, vec![AppCommand::FetchCatalog]);
    assert!(state.snapshot().is_busy);
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));

    let update_snap = state.snapshot();
    assert_eq!(update_snap.update_available, Some(true));
    assert!(!update_snap.is_up_to_date);
    assert_eq!(
        update_snap.version_comparison_text,
        "Update available: 0x0510 → 0x0630"
    );

    // After flashing to 0x0630, version comparison reports Up to date
    state.update(AppEvent::ContinueToConfirmStep);
    state.update(AppEvent::SetWarningAcknowledged(true));
    state.update(AppEvent::StartUpdate);
    state.update(AppEvent::UpdateFinished(Ok(UpdateOutcome::verified(
        0x0630,
    ))));

    let done = state.snapshot();
    assert_eq!(done.update_available, Some(false));
    assert!(done.is_up_to_date);
    assert_eq!(done.version_comparison_text, "Up to date (0x0630)");
    assert!(done.has_cached_firmware);
    assert_eq!(done.cached_firmware_suggested_name, "FWUPDATE0004_0630.bin");
    assert!(done.can_save_cached_firmware);

    // SaveCachedFirmware command and completion banner
    let save_path = PathBuf::from("/tmp/FWUPDATE0004_0630.bin");
    let save_cmds = state.update(AppEvent::SaveCachedFirmware {
        path: save_path.clone(),
    });
    assert_eq!(
        save_cmds,
        vec![AppCommand::SaveCachedFirmware {
            target_id: "0004".to_string(),
            version: 0x0630,
            path: save_path.clone(),
        }]
    );
    state.update(AppEvent::CachedFirmwareSaved {
        path: save_path,
        result: Ok(()),
    });
    assert_eq!(
        state.snapshot().banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );
}
