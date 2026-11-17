// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use dualsense_core::{DEFAULT_PID, DUALSENSE_EDGE_PID};
use dualsense_ui::{AppEvent, AppState, BannerSeverity};

use super::{sample_hid_device, sample_inspection};
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
    assert!(!adapted.can_continue_to_firmware);
    assert!(!adapted.can_start_update);
    assert!(!adapted.has_flash_report);
    assert!(!adapted.flash_report_succeeded);
    assert_eq!(adapted.flash_report_title, "");
    assert_eq!(adapted.flash_report_markdown, "");
    assert_eq!(adapted.flash_report_issue_url, "");
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
