use std::path::PathBuf;

use dualsense_core::{
    ControllerHardwareInfo, CoreError, DUALSENSE_EDGE_PID, DownloadProgress, FirmwareCatalog,
    FirmwareInfo, FirmwareRelease, HidDeviceInfo, UpdateProgress, resolve_controller_target,
};

use super::progress::ProgressSnapshot;

/// Text displayed to confirm the user understands that DualSense firmware updates cannot be reversed.
pub const IRREVERSIBLE_UPDATE_WARNING: &str = "Firmware updates are permanently written to your DualSense controller and cannot be downgraded. Keep the USB cable firmly connected and do not close this window until the update finishes.";

/// Guided steps in the single-window DualSense firmware updater flow.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AppStep {
    DetectController,
    SelectFirmware,
    ConfirmUpdate,
    Updating,
    Completed,
    Error,
}

impl AppStep {
    /// Zero-based index for stepper/breadcrumb UI components.
    pub fn step_index(self) -> u8 {
        match self {
            Self::DetectController => 0,
            Self::SelectFirmware => 1,
            Self::ConfirmUpdate => 2,
            Self::Updating => 3,
            Self::Completed => 4,
            Self::Error => 5,
        }
    }

    /// Plain-English title for the current step.
    pub fn title(self) -> &'static str {
        match self {
            Self::DetectController => "Connect Controller",
            Self::SelectFirmware => "Choose Firmware",
            Self::ConfirmUpdate => "Review & Confirm",
            Self::Updating => "Updating Firmware",
            Self::Completed => "Update Complete",
            Self::Error => "Needs Attention",
        }
    }
}

/// Severity level for inline status banners (maps naturally to Kirigami `InlineMessage`).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BannerSeverity {
    Info,
    Positive,
    Warning,
    Error,
}

/// Inline status message displayed in the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BannerMessage {
    pub severity: BannerSeverity,
    pub title: String,
    pub text: String,
}

impl BannerMessage {
    pub fn info(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            severity: BannerSeverity::Info,
            title: title.into(),
            text: text.into(),
        }
    }

    pub fn positive(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            severity: BannerSeverity::Positive,
            title: title.into(),
            text: text.into(),
        }
    }

    pub fn warning(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            severity: BannerSeverity::Warning,
            title: title.into(),
            text: text.into(),
        }
    }

    pub fn error(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            severity: BannerSeverity::Error,
            title: title.into(),
            text: text.into(),
        }
    }
}

/// Complete hardware, firmware, and optional serial/color inspection result for a controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInspection {
    pub firmware_info: FirmwareInfo,
    pub hardware: ControllerHardwareInfo,
    pub serial_number: Option<String>,
    pub color_name: Option<String>,
}

impl DeviceInspection {
    /// Creates a [`DeviceInspection`] from `0x20` firmware info and resolved hardware details.
    pub fn from_firmware_info(firmware_info: FirmwareInfo, product_id: u16) -> Self {
        let hardware = resolve_controller_target(&firmware_info, product_id);
        Self {
            firmware_info,
            hardware,
            serial_number: None,
            color_name: None,
        }
    }
}

/// Summary of an enumerated DualSense USB HID device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSummary {
    pub path: String,
    pub label: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub is_edge: bool,
    pub serial_number: Option<String>,
    pub color_name: Option<String>,
    pub board_model: String,
    pub fw_series_text: String,
    pub hw_model_text: String,
    pub detected_target_id: Option<String>,
    pub board_supported: bool,
    pub unsupported_board_warning: Option<String>,
    pub firmware_version: Option<u16>,
    pub formatted_firmware_version: String,
    pub build_date: Option<String>,
    pub build_time: Option<String>,
}

impl DeviceSummary {
    /// Creates a summary from enumerated HID metadata before `0x20` firmware info is read.
    pub fn from_hid_info(info: &HidDeviceInfo) -> Self {
        Self {
            path: info.path.clone(),
            label: info.display_label(),
            vendor_id: info.vendor_id,
            product_id: info.product_id,
            is_edge: info.product_id == DUALSENSE_EDGE_PID,
            serial_number: non_empty(&info.serial_number),
            color_name: None,
            board_model: "Unknown".to_string(),
            fw_series_text: String::new(),
            hw_model_text: String::new(),
            detected_target_id: None,
            board_supported: false,
            unsupported_board_warning: None,
            firmware_version: None,
            formatted_firmware_version: "Not read yet".to_string(),
            build_date: None,
            build_time: None,
        }
    }

    /// Updates firmware, board, target, serial, and color fields from controller inspection.
    pub fn apply_inspection(&mut self, inspection: &DeviceInspection) {
        self.firmware_version = Some(inspection.firmware_info.firmware_version);
        self.formatted_firmware_version = inspection.firmware_info.formatted_version();
        self.build_date = non_empty(&inspection.firmware_info.build_date);
        self.build_time = non_empty(&inspection.firmware_info.build_time);

        let hw = &inspection.hardware;
        self.board_model = hw.board_name.clone();
        self.fw_series_text = hw.fw_series_hex.clone();
        self.hw_model_text = format!("0x{:08X}", hw.hw_info);
        self.detected_target_id = hw.supported_target_id.map(ToString::to_string);
        self.board_supported = hw.is_officially_supported;
        self.unsupported_board_warning = if hw.is_officially_supported {
            None
        } else {
            let reason = hw.unsupported_reason.as_deref().unwrap_or(
                "This controller hardware revision is not supported by Sony's official PC firmware catalog.",
            );
            Some(format!(
                "{reason} Automatic firmware download and target selection are disabled; you can still install a local .bin firmware file if you have one."
            ))
        };

        if let Some(serial) = inspection.serial_number.as_deref().and_then(non_empty) {
            self.serial_number = Some(serial);
        }
        self.color_name = inspection.color_name.as_deref().and_then(non_empty);
    }

    /// Updates firmware metadata fields from Feature Report `0x20`.
    pub fn apply_firmware_info(&mut self, info: &FirmwareInfo) {
        self.apply_inspection(&DeviceInspection::from_firmware_info(
            info.clone(),
            self.product_id,
        ));
    }

    /// Returns a [`DeviceTarget`] descriptor for background HID operations.
    pub fn target(&self) -> DeviceTarget {
        DeviceTarget {
            path: self.path.clone(),
            vendor_id: self.vendor_id,
            product_id: self.product_id,
        }
    }
}

/// Identifying info used by [`AppCommand`] to open a specific USB HID controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceTarget {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
}

/// Selectable Sony firmware hardware target/board revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetOption {
    pub target_id: String,
    pub model_name: String,
    pub board_revisions: String,
    pub label: String,
    pub tested: bool,
    pub latest_version: Option<u16>,
    pub latest_version_label: String,
    pub download_url: Option<String>,
    pub is_up_to_date: Option<bool>,
}

impl TargetOption {
    pub(super) fn from_known(
        target_id: &str,
        model_name: &str,
        board_revisions: &str,
        tested: bool,
    ) -> Self {
        let label = if board_revisions == model_name {
            model_name.to_string()
        } else {
            format!("{model_name} ({board_revisions})")
        };
        Self {
            target_id: target_id.to_string(),
            model_name: model_name.to_string(),
            board_revisions: board_revisions.to_string(),
            label,
            tested,
            latest_version: None,
            latest_version_label: "Check for updates to view".to_string(),
            download_url: None,
            is_up_to_date: None,
        }
    }

    pub(super) fn apply_release(&mut self, release: &FirmwareRelease) {
        self.latest_version = Some(release.version);
        self.latest_version_label = format!("0x{:04x}", release.version);
        self.download_url = Some(release.download_url.clone());
    }

    pub(super) fn update_comparison(&mut self, current_version: Option<u16>) {
        self.is_up_to_date = match (current_version, self.latest_version) {
            (Some(current), Some(latest)) => Some(current >= latest),
            _ => None,
        };
    }
}

/// Whether the firmware image comes from Sony's official catalog or a local `.bin` file.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FirmwareSourceKind {
    OfficialCatalog,
    LocalFile,
}

/// Validated metadata extracted from a user-selected local `.bin` firmware file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFirmwareInfo {
    pub version: u16,
    pub size_bytes: usize,
}

/// Formatted summary of a validated local `.bin` firmware file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFirmwareSummary {
    pub path: PathBuf,
    pub file_name: String,
    pub version: u16,
    pub formatted_version: String,
    pub size_bytes: usize,
    pub size_label: String,
}

impl LocalFirmwareSummary {
    pub(super) fn from_info(path: PathBuf, info: LocalFirmwareInfo) -> Self {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| path.display().to_string());
        Self {
            path,
            file_name,
            version: info.version,
            formatted_version: format!("0x{:04x}", info.version),
            size_bytes: info.size_bytes,
            size_label: super::progress::format_bytes(info.size_bytes),
        }
    }
}

/// Pre-flash review summary shown in [`AppStep::ConfirmUpdate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateSummary {
    pub controller_label: String,
    pub current_version_label: String,
    pub target_version_label: String,
    pub source_description: String,
    pub is_untested_hardware: bool,
    pub is_same_or_older_version: bool,
}

/// Active background operation tracked by [`super::AppState`].
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BusyOperation {
    ScanningDevices,
    ReadingDeviceInfo,
    CheckingCatalog,
    InspectingLocalFile,
    DownloadingFirmware,
    FlashingFirmware,
}

impl BusyOperation {
    /// Returns `true` if cancelling this operation is safe and supported in the UI.
    pub fn is_cancellable(self) -> bool {
        !matches!(self, Self::FlashingFirmware)
    }
}

/// Commands emitted by [`super::AppState::update`] for the async runtime to execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    ScanDevices,
    ReadDeviceFirmwareInfo {
        device: DeviceTarget,
    },
    FetchCatalog,
    InspectLocalFile {
        path: PathBuf,
    },
    DownloadAndInstall {
        device: DeviceTarget,
        release: FirmwareRelease,
    },
    InstallLocalFirmware {
        device: DeviceTarget,
        path: PathBuf,
    },
    CancelActiveOperation,
}

/// User interactions and background task results fed into [`super::AppState::update`].
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    ScanDevices,
    SelectDevice {
        path: String,
    },
    RefreshDeviceInfo,
    SetFirmwareSourceKind(FirmwareSourceKind),
    SelectTarget {
        target_id: String,
    },
    CheckForUpdates,
    SelectLocalFile {
        path: PathBuf,
    },
    ContinueToFirmwareStep,
    ContinueToConfirmStep,
    GoBack,
    SetWarningAcknowledged(bool),
    StartUpdate,
    RequestCancel,
    DismissError,
    ResetFlow,

    DevicesScanned(std::result::Result<Vec<HidDeviceInfo>, CoreError>),
    DeviceFirmwareInfoLoaded {
        path: String,
        result: std::result::Result<DeviceInspection, CoreError>,
    },
    CatalogLoaded(std::result::Result<FirmwareCatalog, CoreError>),
    LocalFileInspected {
        path: PathBuf,
        result: std::result::Result<LocalFirmwareInfo, CoreError>,
    },
    DownloadProgressUpdated(DownloadProgress),
    UpdateProgressUpdated(UpdateProgress),
    UpdateFinished(std::result::Result<u16, CoreError>),
}

pub(super) fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Helper alias re-exported internally for `ProgressSnapshot` consumers.
pub(super) type _ProgressAnchor = ProgressSnapshot;
