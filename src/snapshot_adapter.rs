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

fn format_banner_display(title: &str, text: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use dualsense_core::{
        CoreError, DEFAULT_PID, DownloadProgress, FirmwareCatalog, FirmwareInfo, FirmwareRelease,
        HidDeviceInfo, UpdatePhase, UpdateProgress, decode_ds5_color, resolve_controller_target,
    };
    use dualsense_ui::{AppEvent, AppState, DeviceInspection, LocalFirmwareInfo};

    fn sample_hid_device(path: &str, pid: u16, serial: &str) -> HidDeviceInfo {
        HidDeviceInfo {
            path: path.to_string(),
            vendor_id: 0x054c,
            product_id: pid,
            interface_number: 3,
            usage_page: 0x0001,
            usage: 0x0005,
            product_string: if pid == 0x0df2 {
                "DualSense Edge Wireless Controller".to_string()
            } else {
                "DualSense Wireless Controller".to_string()
            },
            serial_number: serial.to_string(),
        }
    }

    fn sample_firmware_info(
        build_date: &str,
        build_time: &str,
        version: u16,
        fw_series: u16,
        hw_info: u32,
    ) -> FirmwareInfo {
        let mut raw = vec![0u8; 64];
        raw[0] = 0x20;
        let date_bytes = build_date.as_bytes();
        let date_len = date_bytes.len().min(11);
        raw[1..1 + date_len].copy_from_slice(&date_bytes[..date_len]);
        let time_bytes = build_time.as_bytes();
        let time_len = time_bytes.len().min(8);
        raw[12..12 + time_len].copy_from_slice(&time_bytes[..time_len]);
        raw[22..24].copy_from_slice(&fw_series.to_le_bytes());
        raw[24..28].copy_from_slice(&hw_info.to_le_bytes());
        raw[44..46].copy_from_slice(&version.to_le_bytes());
        FirmwareInfo::from_report(raw).expect("valid firmware info")
    }

    fn sample_inspection(
        build_date: &str,
        build_time: &str,
        version: u16,
        pid: u16,
        fw_series: u16,
        hw_info: u32,
        serial: Option<&str>,
    ) -> DeviceInspection {
        let firmware_info =
            sample_firmware_info(build_date, build_time, version, fw_series, hw_info);
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
        assert!(!adapted.target_ids.is_empty());
        assert_eq!(adapted.selected_target_id, "0004");
        assert!(adapted.selected_target_index >= 0);
    }

    #[test]
    fn device_scan_and_firmware_info_populate_flat_device_fields() {
        let mut state = AppState::new();
        state.update(AppEvent::DevicesScanned(Ok(vec![
            sample_hid_device("/dev/hidraw0", DEFAULT_PID, "ABC123"),
            sample_hid_device("/dev/hidraw1", 0x0df2, "EDGE99"),
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
    fn catalog_and_local_file_selection_populate_firmware_and_review_fields() {
        let mut state = AppState::new();
        state.update(AppEvent::DevicesScanned(Ok(vec![sample_hid_device(
            "/dev/hidraw0",
            DEFAULT_PID,
            "SN1",
        )])));
        state.update(AppEvent::DeviceFirmwareInfoLoaded {
            path: "/dev/hidraw0".to_string(),
            result: Ok(sample_inspection(
                "Jan 01 2024",
                "00:00:00",
                0x0500,
                DEFAULT_PID,
                0x0004,
                0x0000_0411,
                None,
            )),
        });
        state.update(AppEvent::ContinueToFirmwareStep);
        state.update(AppEvent::CheckForUpdates);
        state.update(AppEvent::CatalogLoaded(Ok(FirmwareCatalog {
            application_latest_version: Some("2.2.1.2".to_string()),
            releases: vec![FirmwareRelease {
                target_id: "0004".to_string(),
                version: 0x0630,
                version_raw: "0x0630".to_string(),
                download_url: "https://fwupdater.dl.playstation.net/fwupdater/fwupdate0004/0x0630/FWUPDATE0004.bin".to_string(),
                target_info: None,
            }],
        })));

        let adapted = AdaptedSnapshot::from_ui_snapshot(&state.snapshot());
        assert_eq!(adapted.step_index, 1);
        assert!(adapted.catalog_checked);
        assert!(!adapted.use_local_file);
        assert!(adapted.can_select_target);
        assert_eq!(adapted.selected_target_latest_version, "0x0630");
        assert!(!adapted.selected_target_up_to_date);
        assert!(adapted.target_labels[0].contains("0x0630"));
        assert!(adapted.can_review_update);

        // Local file mode + review step
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
        state.update(AppEvent::CatalogLoaded(Ok(FirmwareCatalog {
            application_latest_version: None,
            releases: vec![FirmwareRelease {
                target_id: "0004".to_string(),
                version: 0x0630,
                version_raw: "0x0630".to_string(),
                download_url: "https://example.invalid/fw.bin".to_string(),
                target_info: None,
            }],
        })));
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
        // Case A: BDM-010
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

        // Case B: Unknown board
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
}
