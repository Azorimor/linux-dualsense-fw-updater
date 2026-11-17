// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use dualsense_core::{
    DEFAULT_PID, DEFAULT_VID, DUALSENSE_EDGE_PID, DownloadProgress, FirmwareCatalog, FirmwareInfo,
    FirmwareRelease, HidDeviceInfo, REPORT_ID_FIRMWARE_INFO, UpdatePhase, UpdateProgress,
    decode_ds5_color, resolve_controller_target,
};
use dualsense_ui::{AppEvent, AppState, DeviceInspection};
use dualsense_updater::snapshot_adapter::{AdaptedSnapshot, BANNER_POSITIVE, BANNER_WARNING};

fn hid_device(path: &str, pid: u16, serial: &str) -> HidDeviceInfo {
    HidDeviceInfo {
        path: path.to_string(),
        vendor_id: DEFAULT_VID,
        product_id: pid,
        interface_number: 3,
        usage_page: 0x0001,
        usage: 0x0005,
        product_string: if pid == DUALSENSE_EDGE_PID {
            "DualSense Edge Wireless Controller".to_string()
        } else {
            "DualSense Wireless Controller".to_string()
        },
        serial_number: serial.to_string(),
    }
}

fn inspect_device(
    version: u16,
    pid: u16,
    fw_series: u16,
    hw_info: u32,
    serial: Option<&str>,
) -> DeviceInspection {
    let mut raw = vec![0u8; 64];
    raw[0] = REPORT_ID_FIRMWARE_INFO;
    raw[1..12].copy_from_slice(b"Oct  1 2024");
    raw[12..20].copy_from_slice(b"12:34:56");
    raw[22..24].copy_from_slice(&fw_series.to_le_bytes());
    raw[24..28].copy_from_slice(&hw_info.to_le_bytes());
    raw[44..46].copy_from_slice(&version.to_le_bytes());

    let firmware_info = FirmwareInfo::from_report(raw).expect("valid firmware info report");
    let hardware = resolve_controller_target(&firmware_info, pid);
    let serial_number = serial.map(ToString::to_string);
    let color_name = serial.and_then(decode_ds5_color).map(ToString::to_string);
    DeviceInspection {
        firmware_info,
        hardware,
        serial_number,
        color_name,
    }
}

fn sample_catalog() -> FirmwareCatalog {
    FirmwareCatalog {
        application_latest_version: Some("2.2.1.2".to_string()),
        releases: vec![
            FirmwareRelease {
                target_id: "0004".to_string(),
                version: 0x0630,
                version_raw: "0x0630".to_string(),
                download_url: "https://fwupdater.dl.playstation.net/fwupdater/fwupdate0004/0x0630/FWUPDATE0004.bin".to_string(),
                target_info: None,
            },
            FirmwareRelease {
                target_id: "000B".to_string(),
                version: 0x0630,
                version_raw: "0x0630".to_string(),
                download_url: "https://fwupdater.dl.playstation.net/fwupdater/fwupdate000B/0x0630/FWUPDATE000B.bin".to_string(),
                target_info: None,
            },
            FirmwareRelease {
                target_id: "0044".to_string(),
                version: 0x0217,
                version_raw: "0x0217".to_string(),
                download_url: "https://fwupdater.dl.playstation.net/fwupdater/fwupdate0044/0x0217/FWUPDATE0044.bin".to_string(),
                target_info: None,
            },
        ],
    }
}

#[test]
fn bdm050_auto_selection_and_full_catalog_update_flow() {
    let mut state = AppState::new();
    state.update(AppEvent::DevicesScanned(Ok(vec![hid_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "",
    )])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(inspect_device(
            0x0520,
            DEFAULT_PID,
            0x000B,
            0x0000_0811,
            Some("F55301K0H14359961"),
        )),
    });

    let step0 = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(step0.step_index, 0);
    assert_eq!(step0.board_model, "BDM-050");
    assert_eq!(step0.color_name, "Midnight Black");
    assert_eq!(step0.detected_target_id, "000B");
    assert_eq!(step0.selected_target_id, "000B");
    assert!(step0.board_supported);
    assert!(step0.can_continue_to_firmware);

    state.update(AppEvent::ContinueToFirmwareStep);
    state.update(AppEvent::CheckForUpdates);
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));

    let step1 = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(step1.step_index, 1);
    assert!(step1.catalog_checked);
    assert_eq!(step1.selected_target_id, "000B");
    assert_eq!(step1.selected_target_latest_version, "0x0630");
    assert!(!step1.selected_target_up_to_date);
    assert!(step1.can_review_update);

    state.update(AppEvent::ContinueToConfirmStep);
    let unack = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(unack.step_index, 2);
    assert!(unack.has_update_summary);
    assert_eq!(unack.summary_current_version, "0x0520");
    assert_eq!(unack.summary_target_version, "0x0630");
    assert!(!unack.can_start_update);

    state.update(AppEvent::SetWarningAcknowledged(true));
    assert!(AdaptedSnapshot::from_ui_snapshot(&state.snapshot()).can_start_update);

    state.update(AppEvent::StartUpdate);
    state.update(AppEvent::DownloadProgressUpdated(DownloadProgress {
        downloaded_bytes: 750,
        total_bytes: Some(1000),
    }));
    let downloading = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(downloading.step_index, 3);
    assert_eq!(downloading.progress_percentage, 75);
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
    assert_eq!(flashing.step_index, 3);
    assert_eq!(flashing.banner_severity, BANNER_WARNING);
    assert!(!flashing.can_cancel);

    state.update(AppEvent::UpdateFinished(Ok(0x0630)));
    let completed = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(completed.step_index, 4);
    assert_eq!(completed.completed_version, "0x0630");
    assert_eq!(completed.banner_severity, BANNER_POSITIVE);
    assert_eq!(completed.progress_percentage, 100);
    assert!(completed.has_flash_report);
    assert!(completed.flash_report_succeeded);
    assert!(
        completed
            .flash_report_title
            .starts_with("[Flash Report] Success")
    );
    assert!(completed.flash_report_markdown.contains("BDM-050"));
    assert!(
        !completed
            .flash_report_markdown
            .contains("F55301K0H14359961")
    );
    assert!(
        completed
            .flash_report_issue_url
            .contains("template=flash_report.md")
    );
}

#[test]
fn unsupported_bdm010_and_dualsense_edge_gating_flow() {
    let mut bdm010_state = AppState::new();
    bdm010_state.update(AppEvent::DevicesScanned(Ok(vec![hid_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "SN010",
    )])));
    bdm010_state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(inspect_device(
            0x0200,
            DEFAULT_PID,
            0x0001,
            0x0000_0300,
            None,
        )),
    });

    let bdm010_step0 = AdaptedSnapshot::from_ui_snapshot(&bdm010_state.snapshot());
    assert_eq!(bdm010_step0.board_model, "BDM-010");
    assert!(!bdm010_step0.board_supported);
    assert!(bdm010_step0.has_unsupported_board_warning);
    assert!(bdm010_step0.can_continue_to_firmware);
    assert!(!bdm010_step0.can_select_target);

    bdm010_state.update(AppEvent::ContinueToFirmwareStep);
    let bdm010_step1 = AdaptedSnapshot::from_ui_snapshot(&bdm010_state.snapshot());
    assert_eq!(bdm010_step1.step_index, 1);
    assert!(!bdm010_step1.can_select_target);
    assert!(!bdm010_step1.can_check_updates);
    assert!(bdm010_step1.can_select_file);
    assert!(!bdm010_step1.can_review_update);

    let mut edge_state = AppState::new();
    edge_state.update(AppEvent::DevicesScanned(Ok(vec![hid_device(
        "/dev/hidraw1",
        DUALSENSE_EDGE_PID,
        "EDGE1",
    )])));
    edge_state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw1".to_string(),
        result: Ok(inspect_device(
            0x0150,
            DUALSENSE_EDGE_PID,
            0x0044,
            0x0000_0400,
            None,
        )),
    });
    edge_state.update(AppEvent::ContinueToFirmwareStep);
    edge_state.update(AppEvent::CheckForUpdates);
    edge_state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));
    edge_state.update(AppEvent::ContinueToConfirmStep);

    let edge_review = AdaptedSnapshot::from_ui_snapshot(&edge_state.snapshot());
    assert!(edge_review.selected_device_is_edge);
    assert_eq!(edge_review.detected_target_id, "0044");
    assert_eq!(edge_review.selected_target_id, "0044");
    assert!(!edge_review.selected_target_tested);
    assert!(edge_review.summary_is_untested);
}

#[test]
fn mid_flow_disconnect_and_replug_resets_step_banner_and_gating() {
    let mut state = AppState::new();
    state.update(AppEvent::DevicesScanned(Ok(vec![hid_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "SN001",
    )])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(inspect_device(
            0x0500,
            DEFAULT_PID,
            0x0004,
            0x0000_0411,
            None,
        )),
    });
    state.update(AppEvent::ContinueToFirmwareStep);
    state.update(AppEvent::CheckForUpdates);
    state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));
    state.update(AppEvent::ContinueToConfirmStep);
    state.update(AppEvent::SetWarningAcknowledged(true));
    state.update(AppEvent::StartUpdate);
    state.update(AppEvent::DownloadProgressUpdated(DownloadProgress {
        downloaded_bytes: 100,
        total_bytes: Some(1000),
    }));
    assert_eq!(
        AdaptedSnapshot::from_ui_snapshot(&state.snapshot()).step_index,
        3
    );

    // Unplug during active download snaps back to Step 0 with disconnect banner and locks gating
    state.update(AppEvent::DevicesPollUpdated(Vec::new()));
    let disconnected = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(disconnected.step_index, 0);
    assert!(!disconnected.has_devices);
    assert_eq!(disconnected.selected_device_index, -1);
    assert_eq!(disconnected.board_model, "");
    assert_eq!(disconnected.color_name, "");
    assert_eq!(disconnected.banner_severity, BANNER_WARNING);
    assert_eq!(disconnected.banner_title, "Controller disconnected");
    assert!(!disconnected.warning_acknowledged);
    assert!(!disconnected.can_continue_to_firmware);
    assert!(!disconnected.can_start_update);
    assert!(!disconnected.can_cancel);

    // Replug recovers automatically on Step 0 once firmware info loads
    state.update(AppEvent::DevicesPollUpdated(vec![hid_device(
        "/dev/hidraw3",
        DEFAULT_PID,
        "",
    )]));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw3".to_string(),
        result: Ok(inspect_device(
            0x0520,
            DEFAULT_PID,
            0x000B,
            0x0000_0811,
            Some("F55301K0H14359961"),
        )),
    });
    let reconnected = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(reconnected.step_index, 0);
    assert!(reconnected.has_devices);
    assert_eq!(reconnected.selected_device_index, 0);
    assert_eq!(reconnected.board_model, "BDM-050");
    assert_eq!(reconnected.color_name, "Midnight Black");
    assert_eq!(reconnected.banner_severity, BANNER_POSITIVE);
    assert!(reconnected.can_continue_to_firmware);
}
