// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Non-identifiable flash run metrics and prefilled GitHub issue report builder.

use std::fmt::Write as _;

use dualsense_core::DUALSENSE_EDGE_PID;

use crate::error_map::FriendlyError;
use crate::viewmodel::types::{DeviceSummary, FirmwareSourceKind};

/// Application version from `Cargo.toml`.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Canonical GitHub repository URL.
pub const GITHUB_REPO_URL: &str = "https://github.com/Azorimor/linux-dualsense-fw-updater";

/// Official project documentation/download website hosted on GitHub Pages.
pub const GITHUB_PAGES_URL: &str = "https://azorimor.github.io/linux-dualsense-fw-updater/";

/// GitHub Issues list URL.
pub const GITHUB_ISSUES_URL: &str = "https://github.com/Azorimor/linux-dualsense-fw-updater/issues";

/// GitHub New Issue creation URL.
pub const GITHUB_NEW_ISSUE_URL: &str =
    "https://github.com/Azorimor/linux-dualsense-fw-updater/issues/new";

/// Issue template filename inside `.github/ISSUE_TEMPLATE/`.
pub const FLASH_REPORT_TEMPLATE: &str = "flash_report.md";

/// GitHub issue label applied in prefilled URLs (ignored by GitHub if caller lacks label permissions).
pub const FLASH_REPORT_LABEL: &str = "flash-report";

const MAX_TECH_DETAIL_CHARS: usize = 300;

/// Operating system and desktop environment metrics (strictly non-identifiable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemEnvironmentInfo {
    pub os_family: String,
    pub distro: String,
    pub kernel: String,
    pub arch: String,
    pub desktop: Option<String>,
    pub session_type: Option<String>,
    pub packaging: String,
}

impl SystemEnvironmentInfo {
    /// Reads current Linux distribution, kernel, architecture, and desktop session metadata.
    pub fn detect() -> Self {
        let os_release = std::fs::read_to_string("/etc/os-release")
            .or_else(|_| std::fs::read_to_string("/usr/lib/os-release"))
            .ok();
        let distro = os_release
            .as_deref()
            .and_then(parse_os_release)
            .unwrap_or_else(|| format!("Unknown ({})", std::env::consts::OS));

        let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unknown".to_string());

        let desktop = read_non_empty_env("XDG_CURRENT_DESKTOP");
        let session_type = read_non_empty_env("XDG_SESSION_TYPE");
        let packaging = if std::env::var_os("APPIMAGE").is_some() {
            "AppImage".to_string()
        } else {
            "Native".to_string()
        };

        Self {
            os_family: std::env::consts::OS.to_string(),
            distro,
            kernel,
            arch: std::env::consts::ARCH.to_string(),
            desktop,
            session_type,
            packaging,
        }
    }

    /// Formats desktop and session info (e.g., `"KDE (wayland)"`).
    pub fn desktop_summary(&self) -> String {
        match (self.desktop.as_deref(), self.session_type.as_deref()) {
            (Some(de), Some(sess)) => format!("{de} ({sess})"),
            (Some(de), None) => de.to_string(),
            (None, Some(sess)) => sess.to_string(),
            (None, None) => "Unknown".to_string(),
        }
    }
}

/// Snapshot of non-identifiable controller & target metadata captured when an update starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingFlashAttempt {
    pub app_version: String,
    pub model_name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub is_edge: bool,
    pub board_model: String,
    pub fw_series_text: String,
    pub hw_model_text: String,
    pub detected_target_id: Option<String>,
    pub board_supported: bool,
    pub color_name: Option<String>,
    pub build_date: Option<String>,
    pub build_time: Option<String>,
    pub previous_firmware_version: String,
    pub target_firmware_version: String,
    pub source_kind: FirmwareSourceKind,
    pub selected_target_id: Option<String>,
    pub is_untested_hardware: bool,
}

impl PendingFlashAttempt {
    /// Captures a privacy-safe hardware and target summary before flashing modifies version state.
    pub fn from_device(
        device: &DeviceSummary,
        source_kind: FirmwareSourceKind,
        selected_target_id: Option<String>,
        target_firmware_version: String,
        is_untested_hardware: bool,
    ) -> Self {
        let model_name = if device.is_edge || device.product_id == DUALSENSE_EDGE_PID {
            "DualSense Edge".to_string()
        } else {
            "DualSense".to_string()
        };

        Self {
            app_version: APP_VERSION.to_string(),
            model_name,
            vendor_id: device.vendor_id,
            product_id: device.product_id,
            is_edge: device.is_edge,
            board_model: non_empty_or_unknown(&device.board_model),
            fw_series_text: non_empty_or_unknown(&device.fw_series_text),
            hw_model_text: non_empty_or_unknown(&device.hw_model_text),
            detected_target_id: device.detected_target_id.clone(),
            board_supported: device.board_supported,
            color_name: device.color_name.clone(),
            build_date: device.build_date.clone(),
            build_time: device.build_time.clone(),
            previous_firmware_version: device.formatted_firmware_version.clone(),
            target_firmware_version,
            source_kind,
            selected_target_id,
            is_untested_hardware,
        }
    }
}

/// Complete, privacy-safe flash outcome metrics and prefilled GitHub issue URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashReport {
    pub succeeded: bool,
    pub failure_stage: Option<String>,
    pub title: String,
    pub markdown: String,
    pub issue_url: String,
}

impl FlashReport {
    /// Builds a completed flash report (`succeeded: true` or `false`) ready for display or GitHub.
    pub fn build(
        attempt: &PendingFlashAttempt,
        sys: &SystemEnvironmentInfo,
        resulting_firmware_version: Option<String>,
        failure_stage: Option<&str>,
        error: Option<&FriendlyError>,
    ) -> Self {
        let succeeded = error.is_none() && failure_stage.is_none();
        let stage = failure_stage.map(ToString::to_string);
        let title = build_issue_title(attempt, succeeded, stage.as_deref());
        let markdown = build_issue_markdown(
            attempt,
            sys,
            succeeded,
            resulting_firmware_version.as_deref(),
            stage.as_deref(),
            error,
        );
        let issue_url = build_flash_report_issue_url(&title, &markdown);

        Self {
            succeeded,
            failure_stage: stage,
            title,
            markdown,
            issue_url,
        }
    }
}

/// Extracts `PRETTY_NAME` or `NAME` + `VERSION_ID` from an `/etc/os-release` string.
pub fn parse_os_release(contents: &str) -> Option<String> {
    let mut pretty = None;
    let mut name = None;
    let mut version_id = None;

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let unquoted = unquote_os_release_value(value);
        if unquoted.is_empty() {
            continue;
        }
        match key.trim() {
            "PRETTY_NAME" => pretty = Some(unquoted),
            "NAME" => name = Some(unquoted),
            "VERSION_ID" => version_id = Some(unquoted),
            _ => {}
        }
    }

    if let Some(p) = pretty {
        return Some(p);
    }
    match (name, version_id) {
        (Some(n), Some(v)) => Some(format!("{n} {v}")),
        (Some(n), None) => Some(n),
        _ => None,
    }
}

/// Encodes `input` as an RFC 3986 percent-encoded URI query component.
pub fn percent_encode_component(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(input.len());
    for &byte in input.as_bytes() {
        if matches!(
            byte,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~'
        ) {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0F) as usize] as char);
        }
    }
    out
}

/// Builds a GitHub `issues/new` URL prefilled with the `flash_report.md` template, label, title, and body.
pub fn build_flash_report_issue_url(title: &str, body: &str) -> String {
    format!(
        "{GITHUB_NEW_ISSUE_URL}?template={FLASH_REPORT_TEMPLATE}&labels={FLASH_REPORT_LABEL}&title={}&body={}",
        percent_encode_component(title),
        percent_encode_component(body),
    )
}

fn build_issue_title(
    attempt: &PendingFlashAttempt,
    succeeded: bool,
    failure_stage: Option<&str>,
) -> String {
    let status = if succeeded {
        "Success".to_string()
    } else if let Some(stage) = failure_stage {
        format!("Failed ({stage})")
    } else {
        "Failed".to_string()
    };

    format!(
        "[Flash Report] {status} - {} ({}) {} -> {}",
        attempt.model_name,
        attempt.board_model,
        attempt.previous_firmware_version,
        attempt.target_firmware_version,
    )
}

fn build_issue_markdown(
    attempt: &PendingFlashAttempt,
    sys: &SystemEnvironmentInfo,
    succeeded: bool,
    resulting_firmware_version: Option<&str>,
    failure_stage: Option<&str>,
    error: Option<&FriendlyError>,
) -> String {
    let mut md = String::new();
    let result_label = if succeeded { "Success" } else { "Failed" };
    let source_label = match attempt.source_kind {
        FirmwareSourceKind::OfficialCatalog => format!(
            "Official Sony catalog (target {})",
            attempt.selected_target_id.as_deref().unwrap_or("Unknown")
        ),
        FirmwareSourceKind::LocalFile => "Local .bin firmware file".to_string(),
    };
    let build_stamp = match (&attempt.build_date, &attempt.build_time) {
        (Some(d), Some(t)) => format!("{d} {t}"),
        (Some(d), None) => d.clone(),
        (None, Some(t)) => t.clone(),
        (None, None) => "Unknown".to_string(),
    };

    let _ = writeln!(md, "### Flash Summary");
    let _ = writeln!(md, "- **Result:** {result_label}");
    let _ = writeln!(md, "- **App Version:** {}", attempt.app_version);
    let _ = writeln!(md, "- **Firmware Source:** {source_label}");
    let _ = writeln!(
        md,
        "- **Untested Hardware Flag:** {}",
        if attempt.is_untested_hardware {
            "Yes"
        } else {
            "No"
        }
    );
    let _ = writeln!(
        md,
        "- **Previous Firmware Version:** {}",
        attempt.previous_firmware_version
    );
    let _ = writeln!(
        md,
        "- **Target Firmware Version:** {}",
        attempt.target_firmware_version
    );
    let _ = writeln!(
        md,
        "- **Resulting Firmware Version:** {}",
        resulting_firmware_version.unwrap_or("Unchanged / Not verified")
    );

    let _ = writeln!(md, "\n### Controller Hardware (Non-Identifiable)");
    let _ = writeln!(
        md,
        "- **Model:** {} (`{:04x}:{:04x}`)",
        attempt.model_name, attempt.vendor_id, attempt.product_id
    );
    let _ = writeln!(md, "- **Board Revision:** {}", attempt.board_model);
    let _ = writeln!(
        md,
        "- **Detected Target ID:** {}",
        attempt.detected_target_id.as_deref().unwrap_or("Unknown")
    );
    let _ = writeln!(
        md,
        "- **Official Catalog Support:** {}",
        if attempt.board_supported {
            "Supported"
        } else {
            "Unsupported / Custom"
        }
    );
    let _ = writeln!(
        md,
        "- **Hardware Info (`hw_info`):** {}",
        attempt.hw_model_text
    );
    let _ = writeln!(
        md,
        "- **Firmware Series (`fw_series`):** {}",
        attempt.fw_series_text
    );
    let _ = writeln!(
        md,
        "- **Shell Color:** {}",
        attempt.color_name.as_deref().unwrap_or("Unknown")
    );
    let _ = writeln!(md, "- **Controller Build Date/Time:** {build_stamp}");

    let _ = writeln!(md, "\n### Operating System & Environment");
    let _ = writeln!(md, "- **OS / Distribution:** {}", sys.distro);
    let _ = writeln!(md, "- **Kernel:** {}", sys.kernel);
    let _ = writeln!(md, "- **Architecture:** {}", sys.arch);
    let _ = writeln!(md, "- **Desktop / Session:** {}", sys.desktop_summary());
    let _ = writeln!(md, "- **Package Format:** {}", sys.packaging);

    if !succeeded {
        let _ = writeln!(md, "\n### Error Details");
        let _ = writeln!(md, "- **Stage:** {}", failure_stage.unwrap_or("Unknown"));
        if let Some(err) = error {
            let _ = writeln!(
                md,
                "- **Summary:** {} — {}",
                err.error_title, err.error_message
            );
            let sanitized_detail =
                sanitize_and_truncate(&err.technical_detail, MAX_TECH_DETAIL_CHARS);
            let _ = writeln!(md, "- **Technical Detail:** `{sanitized_detail}`");
        }
    }

    let _ = writeln!(md, "\n### Additional Notes");
    let _ = write!(
        md,
        "_Optional: describe USB cable/port setup or controller behavior after this run._"
    );

    md
}

fn sanitize_and_truncate(detail: &str, max_chars: usize) -> String {
    let single_line: String = detail
        .chars()
        .map(|c| {
            if matches!(c, '\r' | '\n' | '`') {
                ' '
            } else {
                c
            }
        })
        .collect();
    let redacted = redact_home_segments(single_line.trim());
    let count = redacted.chars().count();
    if count <= max_chars {
        redacted
    } else {
        let prefix: String = redacted.chars().take(max_chars).collect();
        format!("{prefix}...")
    }
}

fn redact_home_segments(input: &str) -> String {
    let mut out = input.to_string();
    for prefix in ["/home/", "/Users/"] {
        while let Some(idx) = out.find(prefix) {
            let after = idx + prefix.len();
            let user_len = out[after..]
                .find(['/', ' ', '\t'])
                .unwrap_or(out.len() - after);
            out.replace_range(idx..after + user_len, "~");
        }
    }
    out
}

fn unquote_os_release_value(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed[1..trimmed.len() - 1].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn read_non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn non_empty_or_unknown(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "Unknown".to_string()
    } else {
        trimmed.to_string()
    }
}
