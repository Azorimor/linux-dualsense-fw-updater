// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Pure Rust adapter converting [`dualsense_ui::UiSnapshot`] and QML inputs
//! into flat, headless-testable state for the `cxx-qt` bridge.

use std::path::PathBuf;

use dualsense_ui::{
    AppStep, BannerSeverity, DeviceSummary, FirmwareSourceKind, ProgressStage, TargetOption,
    UiSnapshot,
};

/// Integer severity code mapped to `Kirigami.MessageType` in QML.
pub const BANNER_INFO: i32 = 0;
/// Positive/success banner severity mapped to `Kirigami.MessageType.Positive`.
pub const BANNER_POSITIVE: i32 = 1;
/// Warning banner severity mapped to `Kirigami.MessageType.Warning`.
pub const BANNER_WARNING: i32 = 2;
/// Error banner severity mapped to `Kirigami.MessageType.Error`.
pub const BANNER_ERROR: i32 = 3;

/// Flat, presentation-ready representation of [`UiSnapshot`].
///
/// All fields use plain Rust primitives and `Vec<String>` so conversions can be
/// exhaustively unit-tested without starting a Qt event loop or display server.
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptedSnapshot {
    pub step_index: i32,
    pub step_title: String,
    pub step_subtitle: String,

    pub device_paths: Vec<String>,
    pub device_labels: Vec<String>,
    pub device_details: Vec<String>,
    pub device_count: i32,
    pub has_devices: bool,
    pub selected_device_index: i32,
    pub selected_device_path: String,
    pub selected_device_label: String,
    pub selected_device_firmware: String,
    pub selected_device_build_info: String,
    pub selected_device_is_edge: bool,
    pub board_model: String,
    pub fw_series_text: String,
    pub hw_model_text: String,
    pub color_name: String,
    pub serial_number: String,
    pub detected_target_id: String,
    pub board_supported: bool,
    pub has_unsupported_board_warning: bool,
    pub unsupported_board_warning: String,

    pub use_local_file: bool,
    pub target_ids: Vec<String>,
    pub target_labels: Vec<String>,
    pub target_version_labels: Vec<String>,
    pub selected_target_index: i32,
    pub selected_target_id: String,
    pub selected_target_label: String,
    pub selected_target_latest_version: String,
    pub selected_target_tested: bool,
    pub selected_target_up_to_date: bool,
    pub catalog_checked: bool,

    pub has_local_firmware: bool,
    pub local_firmware_name: String,
    pub local_firmware_version: String,
    pub local_firmware_size: String,
    pub local_firmware_path: String,

    pub warning_acknowledged: bool,
    pub irreversible_warning: String,
    pub has_update_summary: bool,
    pub summary_controller: String,
    pub summary_current_version: String,
    pub summary_target_version: String,
    pub summary_source: String,
    pub summary_is_untested: bool,
    pub summary_is_same_or_older: bool,
    pub completed_version: String,

    pub has_banner: bool,
    pub banner_severity: i32,
    pub banner_title: String,
    pub banner_text: String,
    pub banner_display_text: String,

    pub progress_visible: bool,
    pub progress_indeterminate: bool,
    pub progress_value: f64,
    pub progress_percentage: i32,
    pub status_title: String,
    pub status_detail: String,

    pub has_error: bool,
    pub error_title: String,
    pub error_message: String,
    pub recovery_hint: String,

    pub is_busy: bool,
    pub can_scan: bool,
    pub can_continue_to_firmware: bool,
    pub can_select_target: bool,
    pub can_check_updates: bool,
    pub can_select_file: bool,
    pub can_review_update: bool,
    pub can_start_update: bool,
    pub can_cancel: bool,
    pub can_go_back: bool,
}

impl AdaptedSnapshot {
    /// Converts a [`UiSnapshot`] into flat bridge properties.
    pub fn from_ui_snapshot(snapshot: &UiSnapshot) -> Self {
        let device_paths: Vec<String> = snapshot.devices.iter().map(|d| d.path.clone()).collect();
        let device_labels: Vec<String> = snapshot.devices.iter().map(|d| d.label.clone()).collect();
        let device_details: Vec<String> =
            snapshot.devices.iter().map(format_device_detail).collect();
        let device_count = snapshot.devices.len() as i32;
        let has_devices = !snapshot.devices.is_empty();

        let selected_device_path = snapshot.selected_device_path.clone().unwrap_or_default();
        let selected_device_index = snapshot
            .selected_device_path
            .as_deref()
            .and_then(|path| snapshot.devices.iter().position(|d| d.path == path))
            .map(|idx| idx as i32)
            .unwrap_or(-1);

        let (
            selected_device_label,
            selected_device_firmware,
            selected_device_build_info,
            selected_device_is_edge,
        ) = match &snapshot.selected_device {
            Some(dev) => (
                dev.label.clone(),
                dev.formatted_firmware_version.clone(),
                format_build_info(dev),
                dev.is_edge,
            ),
            None => (String::new(), String::new(), String::new(), false),
        };

        let target_ids: Vec<String> = snapshot
            .targets
            .iter()
            .map(|t| t.target_id.clone())
            .collect();
        let target_labels: Vec<String> = snapshot.targets.iter().map(format_target_label).collect();
        let target_version_labels: Vec<String> = snapshot
            .targets
            .iter()
            .map(|t| t.latest_version_label.clone())
            .collect();
        let selected_target_index = snapshot
            .targets
            .iter()
            .position(|t| {
                t.target_id
                    .eq_ignore_ascii_case(&snapshot.selected_target_id)
            })
            .map(|idx| idx as i32)
            .unwrap_or(-1);

        let (
            selected_target_label,
            selected_target_latest_version,
            selected_target_tested,
            selected_target_up_to_date,
        ) = match &snapshot.selected_target {
            Some(target) => (
                target.label.clone(),
                target.latest_version_label.clone(),
                target.tested,
                target.is_up_to_date == Some(true),
            ),
            None => (String::new(), String::new(), true, false),
        };

        let (
            has_local_firmware,
            local_firmware_name,
            local_firmware_version,
            local_firmware_size,
            local_firmware_path,
        ) = match &snapshot.local_firmware {
            Some(local) => (
                true,
                local.file_name.clone(),
                local.formatted_version.clone(),
                local.size_label.clone(),
                local.path.display().to_string(),
            ),
            None => (
                false,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ),
        };

        let (
            has_update_summary,
            summary_controller,
            summary_current_version,
            summary_target_version,
            summary_source,
            summary_is_untested,
            summary_is_same_or_older,
        ) = match &snapshot.update_summary {
            Some(summary) => (
                true,
                summary.controller_label.clone(),
                summary.current_version_label.clone(),
                summary.target_version_label.clone(),
                summary.source_description.clone(),
                summary.is_untested_hardware,
                summary.is_same_or_older_version,
            ),
            None => (
                false,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                false,
                false,
            ),
        };

        let (has_banner, banner_severity, banner_title, banner_text, banner_display_text) =
            match &snapshot.banner {
                Some(banner) => (
                    true,
                    map_banner_severity(banner.severity),
                    banner.title.clone(),
                    banner.text.clone(),
                    format_banner_display(&banner.title, &banner.text),
                ),
                None => (
                    false,
                    BANNER_INFO,
                    String::new(),
                    String::new(),
                    String::new(),
                ),
            };

        let progress_visible = snapshot.is_busy || snapshot.progress.stage != ProgressStage::Idle;
        let progress_percentage = snapshot.progress.percentage.map(i32::from).unwrap_or(-1);
        let progress_value = snapshot
            .progress
            .percentage
            .map(|p| f64::from(p) / 100.0)
            .unwrap_or(0.0);

        let unsupported_board_warning = snapshot
            .unsupported_board_warning
            .clone()
            .unwrap_or_default();
        let has_unsupported_board_warning = !unsupported_board_warning.is_empty();

        Self {
            step_index: i32::from(snapshot.step_index),
            step_title: snapshot.step_title.clone(),
            step_subtitle: step_subtitle(snapshot.step).to_string(),
            device_paths,
            device_labels,
            device_details,
            device_count,
            has_devices,
            selected_device_index,
            selected_device_path,
            selected_device_label,
            selected_device_firmware,
            selected_device_build_info,
            selected_device_is_edge,
            board_model: snapshot.board_model.clone(),
            fw_series_text: snapshot.fw_series_text.clone(),
            hw_model_text: snapshot.hw_model_text.clone(),
            color_name: snapshot.color_name.clone().unwrap_or_default(),
            serial_number: snapshot.serial_number.clone().unwrap_or_default(),
            detected_target_id: snapshot.detected_target_id.clone().unwrap_or_default(),
            board_supported: snapshot.board_supported,
            has_unsupported_board_warning,
            unsupported_board_warning,
            use_local_file: snapshot.source_kind == FirmwareSourceKind::LocalFile,
            target_ids,
            target_labels,
            target_version_labels,
            selected_target_index,
            selected_target_id: snapshot.selected_target_id.clone(),
            selected_target_label,
            selected_target_latest_version,
            selected_target_tested,
            selected_target_up_to_date,
            catalog_checked: snapshot.catalog_checked,
            has_local_firmware,
            local_firmware_name,
            local_firmware_version,
            local_firmware_size,
            local_firmware_path,
            warning_acknowledged: snapshot.warning_acknowledged,
            irreversible_warning: snapshot.irreversible_warning.clone(),
            has_update_summary,
            summary_controller,
            summary_current_version,
            summary_target_version,
            summary_source,
            summary_is_untested,
            summary_is_same_or_older,
            completed_version: snapshot.completed_version.clone().unwrap_or_default(),
            has_banner,
            banner_severity,
            banner_title,
            banner_text,
            banner_display_text,
            progress_visible,
            progress_indeterminate: snapshot.progress.indeterminate,
            progress_value,
            progress_percentage,
            status_title: snapshot.progress.status_text.clone(),
            status_detail: snapshot.progress.detail_text.clone(),
            has_error: snapshot.error.is_some(),
            error_title: snapshot.error_title.clone(),
            error_message: snapshot.error_message.clone(),
            recovery_hint: snapshot.recovery_hint.clone(),
            is_busy: snapshot.is_busy,
            can_scan: snapshot.can_scan,
            can_continue_to_firmware: snapshot.can_continue_to_firmware,
            can_select_target: snapshot.can_select_target,
            can_check_updates: snapshot.can_check_updates,
            can_select_file: snapshot.can_select_file,
            can_review_update: snapshot.can_review_update,
            can_start_update: snapshot.can_start_update,
            can_cancel: snapshot.can_cancel,
            can_go_back: snapshot.can_go_back,
        }
    }
}

/// Maps [`BannerSeverity`] into an integer constant matching `Kirigami.MessageType`.
pub fn map_banner_severity(severity: BannerSeverity) -> i32 {
    match severity {
        BannerSeverity::Info => BANNER_INFO,
        BannerSeverity::Positive => BANNER_POSITIVE,
        BannerSeverity::Warning => BANNER_WARNING,
        BannerSeverity::Error => BANNER_ERROR,
    }
}

/// Parses a QML `FileDialog` selection (`file://` URL or filesystem path) into a [`PathBuf`].
///
/// Rejects empty strings, non-`file://` schemes (such as `http://`), and remote authorities.
pub fn parse_file_dialog_selection(input: &str) -> Option<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("file://") {
        let raw_path = if let Some(after_localhost) = rest.strip_prefix("localhost/") {
            format!("/{after_localhost}")
        } else if rest.starts_with('/') {
            rest.to_string()
        } else {
            return None;
        };

        let decoded = percent_decode_utf8(&raw_path);
        let normalized = normalize_windows_drive_prefix(&decoded);
        if normalized.is_empty() {
            return None;
        }
        return Some(PathBuf::from(normalized));
    }

    if trimmed.contains("://") {
        return None;
    }

    Some(PathBuf::from(trimmed))
}

fn step_subtitle(step: AppStep) -> &'static str {
    match step {
        AppStep::DetectController => {
            "Plug your DualSense controller into a USB port using a data cable."
        }
        AppStep::SelectFirmware => {
            "Download the latest official Sony firmware or choose a local .bin file."
        }
        AppStep::ConfirmUpdate => {
            "Confirm your controller details and read the safety notice before updating."
        }
        AppStep::Updating => "Keep your DualSense connected while the firmware is installed.",
        AppStep::Completed => "Your controller is updated and ready to use.",
        AppStep::Error => "Review the message below and try the suggested step.",
    }
}

fn format_device_detail(dev: &DeviceSummary) -> String {
    let mut parts = vec![format!("Firmware: {}", dev.formatted_firmware_version)];
    if dev.firmware_version.is_some() && !dev.board_model.trim().is_empty() {
        parts.push(format!("Board: {}", dev.board_model.trim()));
    }
    if let Some(color) = dev
        .color_name
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
    {
        parts.push(format!("Color: {color}"));
    }
    if let Some(serial) = dev
        .serial_number
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        parts.push(format!("Serial: {serial}"));
    }
    if dev.is_edge {
        parts.push("DualSense Edge (experimental)".to_string());
    }
    parts.join(" • ")
}

fn format_build_info(dev: &DeviceSummary) -> String {
    match (&dev.build_date, &dev.build_time) {
        (Some(date), Some(time)) => format!("Built {date} {time}"),
        (Some(date), None) => format!("Built {date}"),
        _ => String::new(),
    }
}

fn format_target_label(target: &TargetOption) -> String {
    match target.latest_version {
        Some(ver) => format!("{} — 0x{ver:04x}", target.label),
        None => target.label.clone(),
    }
}

/// Formats banner title and body text into a single display line.
pub fn format_banner_display(title: &str, text: &str) -> String {
    match (title.trim(), text.trim()) {
        ("", "") => String::new(),
        (t, "") => t.to_string(),
        ("", body) => body.to_string(),
        (t, body) => format!("{t}: {body}"),
    }
}

fn normalize_windows_drive_prefix(path: &str) -> &str {
    let bytes = path.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' {
        &path[1..]
    } else {
        path
    }
}

fn percent_decode_utf8(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push((hi << 4) | lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }

    String::from_utf8(out)
        .unwrap_or_else(|err| String::from_utf8_lossy(err.as_bytes()).into_owned())
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
