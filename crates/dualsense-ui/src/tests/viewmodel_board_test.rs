// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use dualsense_core::DEFAULT_PID;

use super::{sample_catalog, sample_hid_device, sample_inspection};
use crate::viewmodel::{
    AppCommand, AppEvent, AppState, BannerSeverity, FirmwareSourceKind, LocalFirmwareInfo,
};

#[test]
fn bdm050_with_serial_auto_selects_target_and_gates_local_file() {
    let mut state = AppState::new();
    let d1 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "");
    state.update(AppEvent::DevicesScanned(Ok(vec![d1])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_inspection(
            0x0520,
            DEFAULT_PID,
            0x000B,
            0x0000_0811,
            Some("F55301K0H14359961"),
        )),
    });

    let snap = state.snapshot();
    assert_eq!(snap.board_model, "BDM-050");
    assert_eq!(snap.fw_series_text, "0x000B");
    assert_eq!(snap.hw_model_text, "0x00000811");
    assert_eq!(snap.color_name.as_deref(), Some("Midnight Black"));
    assert_eq!(snap.serial_number.as_deref(), Some("F55301K0H14359961"));
    assert_eq!(snap.detected_target_id.as_deref(), Some("000B"));
    assert!(snap.board_supported);
    assert!(snap.unsupported_board_warning.is_none());
    assert_eq!(snap.selected_target_id, "000B");
    assert!(snap.can_select_target);

    let dev = snap.selected_device.expect("selected device");
    assert_eq!(dev.board_model, "BDM-050");
    assert_eq!(dev.fw_series_text, "0x000B");
    assert_eq!(dev.hw_model_text, "0x00000811");
    assert_eq!(dev.color_name.as_deref(), Some("Midnight Black"));
    assert_eq!(dev.serial_number.as_deref(), Some("F55301K0H14359961"));
    assert_eq!(dev.detected_target_id.as_deref(), Some("000B"));
    assert!(dev.board_supported);
    assert!(dev.unsupported_board_warning.is_none());

    state.update(AppEvent::ContinueToFirmwareStep);
    assert!(state.snapshot().catalog_checking);
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));
    let step2 = state.snapshot();
    assert!(!step2.catalog_checking);
    assert!(step2.can_select_target);
    assert!(step2.can_check_updates);
    assert!(step2.can_select_file);

    // Switching to local file disables target dropdown while keeping file selection enabled
    state.update(AppEvent::SetFirmwareSourceKind(
        FirmwareSourceKind::LocalFile,
    ));
    let local_snap = state.snapshot();
    assert!(!local_snap.can_select_target);
    assert!(local_snap.can_select_file);

    state.update(AppEvent::SetFirmwareSourceKind(
        FirmwareSourceKind::OfficialCatalog,
    ));
    assert!(state.snapshot().can_select_target);
}

#[test]
fn unsupported_bdm010_and_unknown_boards_disable_target_and_update_check() {
    // Case A: BDM-010
    let mut state = AppState::new();
    let d_010 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "");
    state.update(AppEvent::DevicesScanned(Ok(vec![d_010])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_inspection(
            0x0200,
            DEFAULT_PID,
            0x0001,
            0x0000_0300,
            None,
        )),
    });

    let snap = state.snapshot();
    assert_eq!(snap.board_model, "BDM-010");
    assert_eq!(snap.fw_series_text, "0x0001");
    assert_eq!(snap.hw_model_text, "0x00000300");
    assert!(snap.detected_target_id.is_none());
    assert!(!snap.board_supported);
    assert!(!snap.can_select_target);
    assert!(!snap.can_check_updates);
    assert_eq!(
        snap.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Warning)
    );
    let warning = snap
        .unsupported_board_warning
        .as_deref()
        .expect("warning populated");
    assert!(warning.contains("BDM-010"));
    assert!(warning.contains("disabled"));

    state.update(AppEvent::ContinueToFirmwareStep);
    let step2 = state.snapshot();
    assert!(!step2.can_select_target);
    assert!(!step2.can_check_updates);
    assert!(step2.can_select_file);

    // Target selection and catalog update check are guarded when !board_supported
    assert!(
        state
            .update(AppEvent::SelectTarget {
                target_id: "000B".to_string(),
            })
            .is_empty()
    );
    assert_eq!(state.snapshot().selected_target_id, "0004");
    assert!(state.update(AppEvent::CheckForUpdates).is_empty());
    assert!(!state.snapshot().is_busy);

    // Even if catalog data exists, OfficialCatalog is not ready for unsupported boards
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));
    assert!(!state.snapshot().can_review_update);

    // Local file remains usable on unsupported boards (surfaces Warning, not Incompatible)
    let path = PathBuf::from("/tmp/custom.bin");
    let cmds = state.update(AppEvent::SelectLocalFile { path: path.clone() });
    assert_eq!(
        cmds,
        vec![AppCommand::InspectLocalFile { path: path.clone() }]
    );
    state.update(AppEvent::LocalFileInspected {
        path,
        result: Ok(LocalFirmwareInfo::official(
            0x0250,
            262_144,
            DEFAULT_PID,
            "0004",
        )),
    });
    assert!(state.snapshot().can_review_update);
    assert!(state.snapshot().local_firmware_compatible);
    assert!(state.snapshot().local_firmware_warning.is_some());

    // Case B: Unknown board (zeroed hw_info and fw_series)
    let mut unknown_state = AppState::new();
    let d_unknown = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "");
    unknown_state.update(AppEvent::DevicesScanned(Ok(vec![d_unknown])));
    unknown_state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_inspection(0x0500, DEFAULT_PID, 0, 0, None)),
    });
    let unknown_snap = unknown_state.snapshot();
    assert_eq!(unknown_snap.board_model, "Unknown");
    assert_eq!(unknown_snap.fw_series_text, "0x0000");
    assert_eq!(unknown_snap.hw_model_text, "0x00000000");
    assert!(!unknown_snap.board_supported);
    assert!(!unknown_snap.can_select_target);
    assert!(!unknown_snap.can_check_updates);
    assert!(unknown_snap.unsupported_board_warning.is_some());
}

#[test]
fn switching_between_supported_and_unsupported_devices_updates_gating() {
    let mut state = AppState::new();
    let d_060 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "");
    let d_010 = sample_hid_device("/dev/hidraw1", DEFAULT_PID, "");
    state.update(AppEvent::DevicesScanned(Ok(vec![d_060, d_010])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_inspection(
            0x0641,
            DEFAULT_PID,
            0x000E,
            0x0000_1100,
            Some("F55302K0H14359961"),
        )),
    });
    assert_eq!(state.snapshot().board_model, "BDM-060M");
    assert_eq!(state.snapshot().selected_target_id, "000E");
    assert_eq!(state.snapshot().color_name.as_deref(), Some("Cosmic Red"));
    assert!(state.snapshot().can_select_target);

    // Switch to second device (BDM-010)
    state.update(AppEvent::SelectDevice {
        path: "/dev/hidraw1".to_string(),
    });
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw1".to_string(),
        result: Ok(sample_inspection(
            0x0200,
            DEFAULT_PID,
            0x0001,
            0x0000_0300,
            None,
        )),
    });
    assert!(!state.snapshot().board_supported);
    assert!(!state.snapshot().can_select_target);
    assert!(state.snapshot().unsupported_board_warning.is_some());

    // Switch back to first device (BDM-060M) without re-probing
    let cmds = state.update(AppEvent::SelectDevice {
        path: "/dev/hidraw0".to_string(),
    });
    assert!(cmds.is_empty());
    let restored = state.snapshot();
    assert_eq!(restored.board_model, "BDM-060M");
    assert_eq!(restored.selected_target_id, "000E");
    assert!(restored.board_supported);
    assert!(restored.can_select_target);
    assert!(restored.unsupported_board_warning.is_none());
}
