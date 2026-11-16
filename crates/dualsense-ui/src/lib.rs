//! Headless-testable ViewModel, HTTP client, and async application controller
//! for the DualSense firmware updater GUI.

pub mod controller;
pub mod error_map;
pub mod http;
pub mod viewmodel;

pub use controller::{AppController, AppRuntime, DefaultAppController, DefaultAppRuntime};
pub use error_map::{FriendlyError, map_core_error};
pub use http::{HttpClientConfig, UreqHttpClient, describe_ureq_error};
pub use viewmodel::{
    AppCommand, AppEvent, AppState, AppStep, BannerMessage, BannerSeverity, BusyOperation,
    DeviceInspection, DeviceSummary, DeviceTarget, FirmwareSourceKind, IRREVERSIBLE_UPDATE_WARNING,
    LocalFirmwareInfo, LocalFirmwareSummary, ProgressSnapshot, ProgressStage, TargetOption,
    UiSnapshot, UpdateSummary, format_bytes,
};
