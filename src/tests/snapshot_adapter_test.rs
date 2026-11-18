// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use dualsense_core::{BatteryChargingState, BatteryStatus, DEFAULT_PID, DUALSENSE_EDGE_PID};
use dualsense_ui::{AppEvent, AppState, BannerSeverity, NoFlashOutcome};

use super::{
    sample_bluetooth_hid_device, sample_hid_device, sample_inspection,
    sample_inspection_with_battery,
};
use crate::qt_bridge::{set_startup_no_flash, startup_no_flash};
use crate::snapshot_adapter::{
    AdaptedSnapshot, BANNER_ERROR, BANNER_INFO, BANNER_POSITIVE, BANNER_WARNING,
    format_banner_display, map_banner_severity, parse_file_dialog_selection,
};

#[test]
fn initial_snapshot_adapts_cleanly() {
    let state = AppState::new();
    let adapted = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());

    assert_eq!(adapted.step_index, 0);
    assert_eq!(adapted.step_title, "Connect Controller");
    assert!(!adapted.step_subtitle.is_empty());
    assert!(!adapted.has_devices);
    assert_eq!(adapted.device_count, 0);
    assert_eq!(adapted.selected_device_index, -1);
    assert_eq!(adapted.board_model, "");
    assert_eq!(adapted.color_name, "");
    assert_eq!(adapted.serial_number, "");
    assert!(!adapted.board_supported);
    assert!(!adapted.has_unsupported_board_warning);
    assert!(!adapted.can_select_target);
    assert!(adapted.has_banner);
    assert_eq!(adapted.banner_severity, BANNER_INFO);
    assert!(
        adapted
            .banner_display_text
            .contains("Connect your DualSense controller")
    );
    assert!(!adapted.progress_visible);
    assert_eq!(adapted.progress_percentage, -1);
    assert!(adapted.can_scan);
    assert!(!adapted.is_flashing);
    assert!(!adapted.bluetooth_only_detected);
    assert_eq!(adapted.bluetooth_device_count, 0);
    assert_eq!(adapted.battery_summary, "");
    assert!(!adapted.has_battery_summary);
    assert!(!adapted.battery_is_low);
    assert!(!adapted.battery_is_critically_low);
    assert_eq!(adapted.battery_warning, "");
    assert!(!adapted.catalog_checking);
    assert!(!adapted.update_available);
    assert!(!adapted.is_up_to_date);
    assert_eq!(adapted.version_comparison_text, "");
    assert!(!adapted.local_firmware_compatible);
    assert_eq!(adapted.local_firmware_warning, "");
    assert_eq!(adapted.local_firmware_error, "");
    assert!(!adapted.post_flash_verified);
    assert_eq!(adapted.completed_verification_note, "");
    assert!(!adapted.has_cached_firmware);
    assert_eq!(adapted.cached_firmware_suggested_name, "");
    assert!(!adapted.can_save_cached_firmware);
    assert!(!adapted.can_continue_to_firmware);
    assert!(!adapted.can_start_update);
    assert!(!adapted.has_flash_report);
    assert!(!adapted.flash_report_succeeded);
    assert_eq!(adapted.flash_report_title, "");
    assert_eq!(adapted.flash_report_markdown, "");
    assert_eq!(adapted.flash_report_issue_url, "");
    assert!(!adapted.no_flash_active);
    assert!(!adapted.target_ids.is_empty());
    assert_eq!(adapted.selected_target_id, "0004");
    assert!(adapted.selected_target_index >= 0);
}

#[test]
fn device_scan_and_firmware_info_populate_flat_device_fields() {
    let mut state = AppState::new();
    state.update(AppEvent::DevicesScanned(Ok(vec![
        sample_hid_device("/dev/hidraw0", DEFAULT_PID, "ABC123"),
        sample_hid_device("/dev/hidraw1", DUALSENSE_EDGE_PID, "EDGE99"),
    ])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_inspection(
            "Jan 15 2024",
            "10:20:30",
            0x0520,
            DEFAULT_PID,
            0x0004,
            0x0000_0411,
            None,
        )),
    });

    let adapted = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert!(adapted.has_devices);
    assert_eq!(adapted.device_count, 2);
    assert_eq!(adapted.selected_device_index, 0);
    assert_eq!(adapted.selected_device_path, "/dev/hidraw0");
    assert_eq!(adapted.selected_device_firmware, "0x0520");
    assert_eq!(
        adapted.selected_device_build_info,
        "Built Jan 15 2024 10:20:30"
    );
    assert!(!adapted.selected_device_is_edge);
    assert_eq!(adapted.board_model, "BDM-020");
    assert_eq!(adapted.fw_series_text, "0x0004");
    assert_eq!(adapted.hw_model_text, "0x00000411");
    assert_eq!(adapted.serial_number, "ABC123");
    assert_eq!(adapted.detected_target_id, "0004");
    assert!(adapted.board_supported);
    assert!(!adapted.has_unsupported_board_warning);
    assert!(adapted.can_select_target);
    assert!(adapted.device_details[0].contains("0x0520"));
    assert!(adapted.device_details[0].contains("BDM-020"));
    assert!(adapted.device_details[0].contains("ABC123"));
    assert!(adapted.device_details[1].contains("DualSense Edge (experimental)"));
    assert_eq!(adapted.banner_severity, BANNER_POSITIVE);
    assert!(adapted.can_continue_to_firmware);
}

#[test]
fn parse_file_dialog_selection_handles_urls_and_plain_paths() {
    assert_eq!(
        parse_file_dialog_selection("file:///home/alex/Downloads/FWUPDATE0004.bin"),
        Some(PathBuf::from("/home/alex/Downloads/FWUPDATE0004.bin"))
    );
    assert_eq!(
        parse_file_dialog_selection("file://localhost/tmp/My%20Firmware%20%231.bin"),
        Some(PathBuf::from("/tmp/My Firmware #1.bin"))
    );
    assert_eq!(
        parse_file_dialog_selection("file:///tmp/%C3%BCber.bin"),
        Some(PathBuf::from("/tmp/über.bin"))
    );
    assert_eq!(
        parse_file_dialog_selection("file:///C:/Users/Alex/fw.bin"),
        Some(PathBuf::from("C:/Users/Alex/fw.bin"))
    );
    assert_eq!(
        parse_file_dialog_selection("/var/tmp/fw.bin"),
        Some(PathBuf::from("/var/tmp/fw.bin"))
    );

    assert_eq!(parse_file_dialog_selection(""), None);
    assert_eq!(parse_file_dialog_selection("   "), None);
    assert_eq!(
        parse_file_dialog_selection("https://example.com/fw.bin"),
        None
    );
    assert_eq!(
        parse_file_dialog_selection("file://remote-host/share/fw.bin"),
        None
    );
}

#[test]
fn banner_severity_and_display_formatting_cover_all_variants() {
    assert_eq!(map_banner_severity(BannerSeverity::Info), BANNER_INFO);
    assert_eq!(
        map_banner_severity(BannerSeverity::Positive),
        BANNER_POSITIVE
    );
    assert_eq!(map_banner_severity(BannerSeverity::Warning), BANNER_WARNING);
    assert_eq!(map_banner_severity(BannerSeverity::Error), BANNER_ERROR);

    assert_eq!(format_banner_display("", ""), "");
    assert_eq!(format_banner_display("Title", ""), "Title");
    assert_eq!(format_banner_display("", "Body"), "Body");
    assert_eq!(format_banner_display("Title", "Body"), "Title: Body");
}

#[test]
fn bdm050_supported_board_adapts_color_and_target_index() {
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

    let adapted = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(adapted.board_model, "BDM-050");
    assert_eq!(adapted.fw_series_text, "0x000B");
    assert_eq!(adapted.hw_model_text, "0x00000811");
    assert_eq!(adapted.color_name, "Midnight Black");
    assert_eq!(adapted.serial_number, "F55301K0H14359961");
    assert_eq!(adapted.detected_target_id, "000B");
    assert!(adapted.board_supported);
    assert!(!adapted.has_unsupported_board_warning);
    assert_eq!(adapted.unsupported_board_warning, "");
    assert!(adapted.can_select_target);
    assert_eq!(adapted.selected_target_id, "000B");
    assert!(adapted.selected_target_index >= 0);
    assert_eq!(
        adapted.target_ids[adapted.selected_target_index as usize],
        "000B"
    );
    assert!(adapted.device_details[0].contains("Board: BDM-050"));
    assert!(adapted.device_details[0].contains("Color: Midnight Black"));
    assert!(adapted.device_details[0].contains("Serial: F55301K0H14359961"));
}

#[test]
fn unsupported_bdm010_and_unknown_boards_adapt_warning_and_disable_target() {
    let mut state = AppState::new();
    state.update(AppEvent::DevicesScanned(Ok(vec![sample_hid_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "",
    )])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_inspection(
            "Jan 01 2021",
            "00:00:00",
            0x0200,
            DEFAULT_PID,
            0x0001,
            0x0000_0300,
            None,
        )),
    });

    let bdm010 = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert_eq!(bdm010.board_model, "BDM-010");
    assert_eq!(bdm010.fw_series_text, "0x0001");
    assert_eq!(bdm010.hw_model_text, "0x00000300");
    assert_eq!(bdm010.detected_target_id, "");
    assert!(!bdm010.board_supported);
    assert!(!bdm010.can_select_target);
    assert!(!bdm010.can_check_updates);
    assert!(bdm010.has_unsupported_board_warning);
    assert!(bdm010.unsupported_board_warning.contains("BDM-010"));
    assert_eq!(bdm010.banner_severity, BANNER_WARNING);

    let mut unknown_state = AppState::new();
    unknown_state.update(AppEvent::DevicesScanned(Ok(vec![sample_hid_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "",
    )])));
    unknown_state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_inspection("", "", 0x0500, DEFAULT_PID, 0, 0, None)),
    });

    let unknown = AdaptedSnapshot::from_ui_snapshot(&unknown_state.snapshot());
    assert_eq!(unknown.board_model, "Unknown");
    assert_eq!(unknown.fw_series_text, "0x0000");
    assert_eq!(unknown.hw_model_text, "0x00000000");
    assert!(!unknown.board_supported);
    assert!(!unknown.can_select_target);
    assert!(!unknown.can_check_updates);
    assert!(unknown.has_unsupported_board_warning);
    assert!(!unknown.unsupported_board_warning.is_empty());
}

#[test]
fn bluetooth_only_detection_and_usb_override_adapt_cleanly() {
    let mut state = AppState::new();
    state.update(AppEvent::DevicesScanned(Ok(vec![
        sample_bluetooth_hid_device("/dev/hidraw9", DEFAULT_PID, "BT-1"),
    ])));

    let bt_only = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert!(!bt_only.has_devices);
    assert_eq!(bt_only.device_count, 0);
    assert!(bt_only.bluetooth_only_detected);
    assert_eq!(bt_only.bluetooth_device_count, 1);
    assert_eq!(bt_only.banner_severity, BANNER_WARNING);
    assert!(bt_only.banner_title.contains("Bluetooth"));
    assert!(!bt_only.can_continue_to_firmware);

    state.update(AppEvent::DevicesScanned(Ok(vec![
        sample_bluetooth_hid_device("/dev/hidraw9", DEFAULT_PID, "BT-1"),
        sample_hid_device("/dev/hidraw0", DEFAULT_PID, "USB-1"),
    ])));
    let with_usb = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert!(with_usb.has_devices);
    assert_eq!(with_usb.device_count, 1);
    assert!(!with_usb.bluetooth_only_detected);
    assert_eq!(with_usb.bluetooth_device_count, 1);
}

#[test]
fn battery_summary_warnings_and_device_details_adapt_cleanly() {
    let mut state = AppState::new();
    state.update(AppEvent::DevicesScanned(Ok(vec![sample_hid_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "",
    )])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_inspection_with_battery(
            "Oct  1 2024",
            "12:34:56",
            0x0520,
            DEFAULT_PID,
            0x000B,
            0x0000_0811,
            Some("F55301K0H14359961"),
            Some(BatteryStatus {
                level: 1,
                percentage: 10,
                charging_state: BatteryChargingState::Discharging,
            }),
        )),
    });

    let critical = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert!(critical.has_battery_summary);
    assert!(critical.battery_summary.contains("10%"));
    assert!(critical.battery_is_low);
    assert!(critical.battery_is_critically_low);
    assert!(critical.battery_warning.contains("critically low"));
    assert!(critical.device_details[0].contains("Firmware: 0x0520"));
    assert!(critical.device_details[0].contains("Board: BDM-050"));
    assert!(critical.device_details[0].contains("Color: Midnight Black"));
    assert!(critical.device_details[0].contains("Battery: "));

    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw0".to_string(),
        result: Ok(sample_inspection_with_battery(
            "Oct  1 2024",
            "12:34:56",
            0x0520,
            DEFAULT_PID,
            0x000B,
            0x0000_0811,
            Some("F55301K0H14359961"),
            Some(BatteryStatus {
                level: 8,
                percentage: 85,
                charging_state: BatteryChargingState::Charging,
            }),
        )),
    });

    let healthy = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
    assert!(healthy.has_battery_summary);
    assert!(!healthy.battery_is_low);
    assert!(!healthy.battery_is_critically_low);
    assert_eq!(healthy.battery_warning, "");
}

#[test]
fn no_flash_flag_and_startup_configuration_adapt_cleanly() {
    let sim_state = AppState::with_no_flash(true);
    let adapted = AdaptedSnapshot::from_ui_snapshot(&sim_state.snapshot());
    assert!(adapted.no_flash_active);

    assert_eq!(startup_no_flash(), None);
    set_startup_no_flash(Some(NoFlashOutcome::Error));
    assert_eq!(startup_no_flash(), Some(NoFlashOutcome::Error));
    set_startup_no_flash(Some(NoFlashOutcome::Success));
    assert_eq!(startup_no_flash(), Some(NoFlashOutcome::Success));
    set_startup_no_flash(None);
    assert_eq!(startup_no_flash(), None);
}
