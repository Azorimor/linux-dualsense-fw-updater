// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Headless-testable ViewModel, HTTP client, and async application controller
//! for the DualSense firmware updater GUI.

pub mod controller;
pub mod error_map;
pub mod flash_report;
pub mod http;
pub mod viewmodel;

#[cfg(test)]
mod tests;

pub use controller::{
    AppController, AppRuntime, DEFAULT_DEVICE_POLL_INTERVAL, DefaultAppController,
    DefaultAppRuntime,
};
pub use error_map::{FriendlyError, map_core_error};
pub use flash_report::{
    APP_VERSION, FLASH_REPORT_LABEL, FLASH_REPORT_TEMPLATE, FlashReport, GITHUB_ISSUES_URL,
    GITHUB_NEW_ISSUE_URL, GITHUB_PAGES_URL, GITHUB_REPO_URL, SystemEnvironmentInfo,
    build_flash_report_issue_url, parse_os_release, percent_encode_component,
};
pub use http::{HttpClientConfig, UreqHttpClient, describe_ureq_error};
pub use viewmodel::{
    AppCommand, AppEvent, AppState, AppStep, BannerMessage, BannerSeverity, BusyOperation,
    DeviceInspection, DeviceSummary, DeviceTarget, FirmwareSourceKind, IRREVERSIBLE_UPDATE_WARNING,
    LocalFirmwareInfo, LocalFirmwareSummary, ProgressSnapshot, ProgressStage, TargetOption,
    UiSnapshot, UpdateSummary, format_bytes,
};
