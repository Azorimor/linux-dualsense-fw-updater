use std::path::PathBuf;

use dualsense_core::{
    ControllerHardwareInfo, CoreError, DUALSENSE_EDGE_PID, DownloadProgress, FirmwareCatalog,
    FirmwareInfo, FirmwareRelease, HidDeviceInfo, KNOWN_TARGETS, UpdatePhase, UpdateProgress,
    find_known_target, resolve_controller_target,
};

use crate::error_map::{FriendlyError, map_core_error};

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
    fn from_known(target_id: &str, model_name: &str, board_revisions: &str, tested: bool) -> Self {
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

    fn apply_release(&mut self, release: &FirmwareRelease) {
        self.latest_version = Some(release.version);
        self.latest_version_label = format!("0x{:04x}", release.version);
        self.download_url = Some(release.download_url.clone());
    }

    fn update_comparison(&mut self, current_version: Option<u16>) {
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
    fn from_info(path: PathBuf, info: LocalFirmwareInfo) -> Self {
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
            size_label: format_bytes(info.size_bytes),
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

/// Coarse stage used to format progress bars and status labels.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ProgressStage {
    Idle,
    Scanning,
    ReadingController,
    CheckingCatalog,
    InspectingFile,
    Downloading,
    StartingUpdate,
    WritingFirmware,
    VerifyingFirmware,
    FinalizingUpdate,
    Completed,
}

/// Formatted progress state exposed to the presentation layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressSnapshot {
    pub stage: ProgressStage,
    pub percentage: Option<u8>,
    pub indeterminate: bool,
    pub status_text: String,
    pub detail_text: String,
    pub current_bytes: usize,
    pub total_bytes: Option<usize>,
}

impl Default for ProgressSnapshot {
    fn default() -> Self {
        Self::idle()
    }
}

impl ProgressSnapshot {
    pub fn idle() -> Self {
        Self {
            stage: ProgressStage::Idle,
            percentage: None,
            indeterminate: false,
            status_text: String::new(),
            detail_text: String::new(),
            current_bytes: 0,
            total_bytes: None,
        }
    }

    pub fn indeterminate(stage: ProgressStage, status_text: impl Into<String>) -> Self {
        Self {
            stage,
            percentage: None,
            indeterminate: true,
            status_text: status_text.into(),
            detail_text: String::new(),
            current_bytes: 0,
            total_bytes: None,
        }
    }

    pub fn from_download(progress: DownloadProgress) -> Self {
        match progress.total_bytes {
            Some(total) if total > 0 => {
                let pct = compute_percentage(progress.downloaded_bytes, total);
                Self {
                    stage: ProgressStage::Downloading,
                    percentage: Some(pct),
                    indeterminate: false,
                    status_text: format!("Downloading firmware… ({pct}%)"),
                    detail_text: format!(
                        "{} of {}",
                        format_bytes(progress.downloaded_bytes),
                        format_bytes(total)
                    ),
                    current_bytes: progress.downloaded_bytes,
                    total_bytes: Some(total),
                }
            }
            _ => Self {
                stage: ProgressStage::Downloading,
                percentage: None,
                indeterminate: true,
                status_text: "Downloading firmware…".to_string(),
                detail_text: format!("{} downloaded", format_bytes(progress.downloaded_bytes)),
                current_bytes: progress.downloaded_bytes,
                total_bytes: progress.total_bytes,
            },
        }
    }

    pub fn from_update(progress: &UpdateProgress) -> Self {
        match progress.phase {
            UpdatePhase::Starting => Self::indeterminate(
                ProgressStage::StartingUpdate,
                "Preparing controller for update…",
            ),
            UpdatePhase::Writing => {
                let pct = (progress.percentage.round().clamp(0.0, 100.0)) as u8;
                Self {
                    stage: ProgressStage::WritingFirmware,
                    percentage: Some(pct),
                    indeterminate: false,
                    status_text: format!("Installing firmware… ({pct}%)"),
                    detail_text: format!(
                        "{} of {} (part {} of {})",
                        format_bytes(progress.current_bytes),
                        format_bytes(progress.total_bytes),
                        progress.current_block,
                        progress.total_blocks
                    ),
                    current_bytes: progress.current_bytes,
                    total_bytes: Some(progress.total_bytes),
                }
            }
            UpdatePhase::Verifying => Self::indeterminate(
                ProgressStage::VerifyingFirmware,
                "Verifying installed firmware…",
            ),
            UpdatePhase::Finalizing => Self::indeterminate(
                ProgressStage::FinalizingUpdate,
                "Finalizing firmware update…",
            ),
            UpdatePhase::Completed => Self::completed(),
        }
    }

    pub fn completed() -> Self {
        Self {
            stage: ProgressStage::Completed,
            percentage: Some(100),
            indeterminate: false,
            status_text: "Firmware update complete (100%)".to_string(),
            detail_text: String::new(),
            current_bytes: 0,
            total_bytes: None,
        }
    }
}

/// Active background operation tracked by [`AppState`].
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

/// Commands emitted by [`AppState::update`] for the async runtime to execute.
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

/// User interactions and background task results fed into [`AppState::update`].
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

/// Read-only snapshot rendered by QML/Kirigami or asserted in headless tests.
#[derive(Debug, Clone, PartialEq)]
pub struct UiSnapshot {
    pub step: AppStep,
    pub step_index: u8,
    pub step_title: String,
    pub devices: Vec<DeviceSummary>,
    pub selected_device_path: Option<String>,
    pub selected_device: Option<DeviceSummary>,
    pub board_model: String,
    pub fw_series_text: String,
    pub hw_model_text: String,
    pub color_name: Option<String>,
    pub serial_number: Option<String>,
    pub detected_target_id: Option<String>,
    pub board_supported: bool,
    pub unsupported_board_warning: Option<String>,
    pub source_kind: FirmwareSourceKind,
    pub targets: Vec<TargetOption>,
    pub selected_target_id: String,
    pub selected_target: Option<TargetOption>,
    pub catalog_checked: bool,
    pub local_firmware: Option<LocalFirmwareSummary>,
    pub warning_acknowledged: bool,
    pub irreversible_warning: String,
    pub update_summary: Option<UpdateSummary>,
    pub completed_version: Option<String>,
    pub banner: Option<BannerMessage>,
    pub progress: ProgressSnapshot,
    pub error: Option<FriendlyError>,
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

/// Pure, deterministic Elm-style state machine for the DualSense firmware updater UI.
#[derive(Debug, Clone)]
pub struct AppState {
    step: AppStep,
    previous_step: AppStep,
    devices: Vec<DeviceSummary>,
    selected_device_path: Option<String>,
    source_kind: FirmwareSourceKind,
    targets: Vec<TargetOption>,
    selected_target_id: String,
    catalog_releases: Vec<FirmwareRelease>,
    catalog_checked: bool,
    local_firmware: Option<LocalFirmwareSummary>,
    warning_acknowledged: bool,
    completed_version: Option<u16>,
    banner: Option<BannerMessage>,
    progress: ProgressSnapshot,
    error: Option<FriendlyError>,
    busy: Option<BusyOperation>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Creates the initial application state with known DualSense hardware targets pre-populated.
    pub fn new() -> Self {
        let targets = KNOWN_TARGETS
            .iter()
            .map(|t| {
                TargetOption::from_known(t.target_id, t.model_name, t.board_revisions, t.tested)
            })
            .collect();

        Self {
            step: AppStep::DetectController,
            previous_step: AppStep::DetectController,
            devices: Vec::new(),
            selected_device_path: None,
            source_kind: FirmwareSourceKind::OfficialCatalog,
            targets,
            selected_target_id: "0004".to_string(),
            catalog_releases: Vec::new(),
            catalog_checked: false,
            local_firmware: None,
            warning_acknowledged: false,
            completed_version: None,
            banner: Some(BannerMessage::info(
                "Connect your DualSense controller",
                "Plug your controller into your PC with a USB cable, then click Scan for Controllers.",
            )),
            progress: ProgressSnapshot::idle(),
            error: None,
            busy: None,
        }
    }

    /// Processes an [`AppEvent`], mutates state deterministically, and returns [`AppCommand`]s to run.
    pub fn update(&mut self, event: AppEvent) -> Vec<AppCommand> {
        match event {
            AppEvent::ScanDevices => self.handle_scan_devices(),
            AppEvent::SelectDevice { path } => self.handle_select_device(&path),
            AppEvent::RefreshDeviceInfo => self.handle_refresh_device_info(),
            AppEvent::SetFirmwareSourceKind(kind) => {
                if self.busy.is_some() {
                    return Vec::new();
                }
                self.source_kind = kind;
                self.warning_acknowledged = false;
                Vec::new()
            }
            AppEvent::SelectTarget { target_id } => {
                if self.busy.is_some() || !self.selected_device_board_supported() {
                    return Vec::new();
                }
                if self
                    .targets
                    .iter()
                    .any(|t| t.target_id.eq_ignore_ascii_case(&target_id))
                {
                    if let Some(canonical) = self
                        .targets
                        .iter()
                        .find(|t| t.target_id.eq_ignore_ascii_case(&target_id))
                    {
                        self.selected_target_id = canonical.target_id.clone();
                    }
                    self.source_kind = FirmwareSourceKind::OfficialCatalog;
                    self.warning_acknowledged = false;
                    self.refresh_catalog_banner();
                }
                Vec::new()
            }
            AppEvent::CheckForUpdates => self.handle_check_for_updates(),
            AppEvent::SelectLocalFile { path } => self.handle_select_local_file(path),
            AppEvent::ContinueToFirmwareStep => {
                if self.busy.is_none()
                    && self.step == AppStep::DetectController
                    && self.selected_device_path.is_some()
                {
                    self.step = AppStep::SelectFirmware;
                    self.warning_acknowledged = false;
                }
                Vec::new()
            }
            AppEvent::ContinueToConfirmStep => {
                if self.busy.is_none()
                    && self.step == AppStep::SelectFirmware
                    && self.selected_device_path.is_some()
                    && self.has_ready_firmware()
                {
                    self.step = AppStep::ConfirmUpdate;
                    self.warning_acknowledged = false;
                }
                Vec::new()
            }
            AppEvent::GoBack => {
                if self.busy.is_some() {
                    return Vec::new();
                }
                match self.step {
                    AppStep::SelectFirmware => self.step = AppStep::DetectController,
                    AppStep::ConfirmUpdate => {
                        self.step = AppStep::SelectFirmware;
                        self.warning_acknowledged = false;
                    }
                    _ => {}
                }
                Vec::new()
            }
            AppEvent::SetWarningAcknowledged(acknowledged) => {
                if self.busy.is_none() && self.step == AppStep::ConfirmUpdate {
                    self.warning_acknowledged = acknowledged;
                }
                Vec::new()
            }
            AppEvent::StartUpdate => self.handle_start_update(),
            AppEvent::RequestCancel => self.handle_request_cancel(),
            AppEvent::DismissError => {
                if self.step == AppStep::Error && self.busy.is_none() {
                    self.error = None;
                    self.step = self.previous_step;
                    self.progress = ProgressSnapshot::idle();
                    self.banner = None;
                }
                Vec::new()
            }
            AppEvent::ResetFlow => {
                if self.busy.is_some() {
                    return Vec::new();
                }
                self.step = AppStep::DetectController;
                self.previous_step = AppStep::DetectController;
                self.error = None;
                self.completed_version = None;
                self.warning_acknowledged = false;
                self.progress = ProgressSnapshot::idle();
                self.banner = None;
                Vec::new()
            }
            AppEvent::DevicesScanned(result) => self.handle_devices_scanned(result),
            AppEvent::DeviceFirmwareInfoLoaded { path, result } => {
                self.handle_device_firmware_info_loaded(&path, result)
            }
            AppEvent::CatalogLoaded(result) => self.handle_catalog_loaded(result),
            AppEvent::LocalFileInspected { path, result } => {
                self.handle_local_file_inspected(path, result)
            }
            AppEvent::DownloadProgressUpdated(progress) => {
                if self.step == AppStep::Updating {
                    self.progress = ProgressSnapshot::from_download(progress);
                }
                Vec::new()
            }
            AppEvent::UpdateProgressUpdated(progress) => {
                if self.step == AppStep::Updating {
                    self.busy = Some(BusyOperation::FlashingFirmware);
                    self.progress = ProgressSnapshot::from_update(&progress);
                    self.banner = Some(BannerMessage::warning(
                        "Do not unplug your controller",
                        "Firmware is being written. Keep the USB cable connected until completion.",
                    ));
                }
                Vec::new()
            }
            AppEvent::UpdateFinished(result) => self.handle_update_finished(result),
        }
    }

    /// Produces a complete [`UiSnapshot`] reflecting the current state and action gating flags.
    pub fn snapshot(&self) -> UiSnapshot {
        let is_busy = self.busy.is_some();
        let selected_device = self.selected_device().cloned();
        let selected_target = self.selected_target().cloned();
        let has_device = selected_device.is_some();
        let has_ready_firmware = self.has_ready_firmware();

        let (
            board_model,
            fw_series_text,
            hw_model_text,
            color_name,
            serial_number,
            detected_target_id,
            board_supported,
            unsupported_board_warning,
        ) = match &selected_device {
            Some(dev) => (
                dev.board_model.clone(),
                dev.fw_series_text.clone(),
                dev.hw_model_text.clone(),
                dev.color_name.clone(),
                dev.serial_number.clone(),
                dev.detected_target_id.clone(),
                dev.board_supported,
                dev.unsupported_board_warning.clone(),
            ),
            None => (
                String::new(),
                String::new(),
                String::new(),
                None,
                None,
                None,
                false,
                None,
            ),
        };

        let (error_title, error_message, recovery_hint) = match &self.error {
            Some(err) => (
                err.error_title.clone(),
                err.error_message.clone(),
                err.recovery_hint.clone(),
            ),
            None => (String::new(), String::new(), String::new()),
        };

        let update_summary = self.build_update_summary();
        let use_local_file = self.source_kind == FirmwareSourceKind::LocalFile;
        let can_scan = !is_busy && self.step != AppStep::Updating;
        let can_continue_to_firmware =
            !is_busy && self.step == AppStep::DetectController && has_device;
        let can_select_target = !is_busy && !use_local_file && board_supported;
        let can_check_updates = !is_busy
            && board_supported
            && matches!(self.step, AppStep::SelectFirmware | AppStep::ConfirmUpdate);
        let can_select_file =
            !is_busy && matches!(self.step, AppStep::SelectFirmware | AppStep::ConfirmUpdate);
        let can_review_update =
            !is_busy && self.step == AppStep::SelectFirmware && has_device && has_ready_firmware;
        let can_start_update = !is_busy
            && self.step == AppStep::ConfirmUpdate
            && has_device
            && has_ready_firmware
            && self.warning_acknowledged;
        let can_cancel = self.busy.is_some_and(BusyOperation::is_cancellable);
        let can_go_back =
            !is_busy && matches!(self.step, AppStep::SelectFirmware | AppStep::ConfirmUpdate);

        UiSnapshot {
            step: self.step,
            step_index: self.step.step_index(),
            step_title: self.step.title().to_string(),
            devices: self.devices.clone(),
            selected_device_path: self.selected_device_path.clone(),
            selected_device,
            board_model,
            fw_series_text,
            hw_model_text,
            color_name,
            serial_number,
            detected_target_id,
            board_supported,
            unsupported_board_warning,
            source_kind: self.source_kind,
            targets: self.targets.clone(),
            selected_target_id: self.selected_target_id.clone(),
            selected_target,
            catalog_checked: self.catalog_checked,
            local_firmware: self.local_firmware.clone(),
            warning_acknowledged: self.warning_acknowledged,
            irreversible_warning: IRREVERSIBLE_UPDATE_WARNING.to_string(),
            update_summary,
            completed_version: self.completed_version.map(|v| format!("0x{v:04x}")),
            banner: self.banner.clone(),
            progress: self.progress.clone(),
            error: self.error.clone(),
            error_title,
            error_message,
            recovery_hint,
            is_busy,
            can_scan,
            can_continue_to_firmware,
            can_select_target,
            can_check_updates,
            can_select_file,
            can_review_update,
            can_start_update,
            can_cancel,
            can_go_back,
        }
    }

    fn handle_scan_devices(&mut self) -> Vec<AppCommand> {
        if self.busy.is_some() || self.step == AppStep::Updating {
            return Vec::new();
        }
        self.error = None;
        self.completed_version = None;
        if matches!(self.step, AppStep::Error | AppStep::Completed) {
            self.step = AppStep::DetectController;
        }
        self.busy = Some(BusyOperation::ScanningDevices);
        self.progress = ProgressSnapshot::indeterminate(
            ProgressStage::Scanning,
            "Looking for connected DualSense controllers…",
        );
        self.banner = None;
        vec![AppCommand::ScanDevices]
    }

    fn handle_select_device(&mut self, path: &str) -> Vec<AppCommand> {
        if self.busy.is_some() {
            return Vec::new();
        }
        let Some(dev) = self.devices.iter().find(|d| d.path == path).cloned() else {
            return Vec::new();
        };
        self.selected_device_path = Some(dev.path.clone());
        self.warning_acknowledged = false;
        self.auto_match_target_for_device(&dev);
        self.refresh_target_comparisons();

        if dev.firmware_version.is_none() {
            self.busy = Some(BusyOperation::ReadingDeviceInfo);
            self.progress = ProgressSnapshot::indeterminate(
                ProgressStage::ReadingController,
                "Reading controller firmware version…",
            );
            return vec![AppCommand::ReadDeviceFirmwareInfo {
                device: dev.target(),
            }];
        }

        if let Some(warning) = &dev.unsupported_board_warning {
            self.banner = Some(BannerMessage::warning(
                "Unsupported controller board",
                warning.clone(),
            ));
        } else {
            self.banner = Some(BannerMessage::positive(
                "Controller ready",
                format!(
                    "Detected current firmware version {}.",
                    dev.formatted_firmware_version
                ),
            ));
        }

        Vec::new()
    }

    fn handle_refresh_device_info(&mut self) -> Vec<AppCommand> {
        if self.busy.is_some() {
            return Vec::new();
        }
        let Some(dev) = self.selected_device().cloned() else {
            return Vec::new();
        };
        self.busy = Some(BusyOperation::ReadingDeviceInfo);
        self.progress = ProgressSnapshot::indeterminate(
            ProgressStage::ReadingController,
            "Reading controller firmware version…",
        );
        vec![AppCommand::ReadDeviceFirmwareInfo {
            device: dev.target(),
        }]
    }

    fn handle_check_for_updates(&mut self) -> Vec<AppCommand> {
        if self.busy.is_some()
            || !self.selected_device_board_supported()
            || !matches!(self.step, AppStep::SelectFirmware | AppStep::ConfirmUpdate)
        {
            return Vec::new();
        }
        self.error = None;
        self.busy = Some(BusyOperation::CheckingCatalog);
        self.progress = ProgressSnapshot::indeterminate(
            ProgressStage::CheckingCatalog,
            "Checking Sony's server for the latest firmware…",
        );
        self.banner = None;
        vec![AppCommand::FetchCatalog]
    }

    fn handle_select_local_file(&mut self, path: PathBuf) -> Vec<AppCommand> {
        if self.busy.is_some()
            || !matches!(self.step, AppStep::SelectFirmware | AppStep::ConfirmUpdate)
        {
            return Vec::new();
        }
        self.source_kind = FirmwareSourceKind::LocalFile;
        self.warning_acknowledged = false;
        self.error = None;
        self.busy = Some(BusyOperation::InspectingLocalFile);
        self.progress = ProgressSnapshot::indeterminate(
            ProgressStage::InspectingFile,
            "Validating firmware file…",
        );
        vec![AppCommand::InspectLocalFile { path }]
    }

    fn handle_start_update(&mut self) -> Vec<AppCommand> {
        if self.busy.is_some() || self.step != AppStep::ConfirmUpdate || !self.warning_acknowledged
        {
            return Vec::new();
        }
        let Some(device) = self.selected_device().map(DeviceSummary::target) else {
            return Vec::new();
        };

        match self.source_kind {
            FirmwareSourceKind::OfficialCatalog => {
                let Some(release) = self.selected_release().cloned() else {
                    return Vec::new();
                };
                self.step = AppStep::Updating;
                self.error = None;
                self.completed_version = None;
                self.busy = Some(BusyOperation::DownloadingFirmware);
                self.progress = ProgressSnapshot::from_download(DownloadProgress {
                    downloaded_bytes: 0,
                    total_bytes: None,
                });
                self.banner = Some(BannerMessage::info(
                    "Downloading official firmware",
                    "You can still cancel safely while the firmware file is downloading.",
                ));
                vec![AppCommand::DownloadAndInstall { device, release }]
            }
            FirmwareSourceKind::LocalFile => {
                let Some(local) = self.local_firmware.as_ref() else {
                    return Vec::new();
                };
                self.step = AppStep::Updating;
                self.error = None;
                self.completed_version = None;
                self.busy = Some(BusyOperation::FlashingFirmware);
                self.progress = ProgressSnapshot::indeterminate(
                    ProgressStage::StartingUpdate,
                    "Preparing controller for update…",
                );
                self.banner = Some(BannerMessage::warning(
                    "Do not unplug your controller",
                    "Firmware is being written. Keep the USB cable connected until completion.",
                ));
                vec![AppCommand::InstallLocalFirmware {
                    device,
                    path: local.path.clone(),
                }]
            }
        }
    }

    fn handle_request_cancel(&mut self) -> Vec<AppCommand> {
        if !self.busy.is_some_and(BusyOperation::is_cancellable) {
            return Vec::new();
        }
        self.banner = Some(BannerMessage::info(
            "Cancelling…",
            "Stopping the current operation cleanly.",
        ));
        vec![AppCommand::CancelActiveOperation]
    }

    fn handle_devices_scanned(
        &mut self,
        result: std::result::Result<Vec<HidDeviceInfo>, CoreError>,
    ) -> Vec<AppCommand> {
        self.busy = None;
        self.progress = ProgressSnapshot::idle();

        match result {
            Ok(infos) => {
                let previous_path = self.selected_device_path.clone();
                self.devices = infos.iter().map(DeviceSummary::from_hid_info).collect();

                if self.devices.is_empty() {
                    self.selected_device_path = None;
                    self.refresh_target_comparisons();
                    self.banner = Some(BannerMessage::warning(
                        "No DualSense controller detected",
                        "Plug your controller directly into your PC using a USB data cable (updates cannot run over Bluetooth), then click Scan Again.",
                    ));
                    return Vec::new();
                }

                let selected_path = previous_path
                    .filter(|prev| self.devices.iter().any(|d| d.path == *prev))
                    .unwrap_or_else(|| self.devices[0].path.clone());
                self.selected_device_path = Some(selected_path.clone());

                if let Some(dev) = self.selected_device().cloned() {
                    self.auto_match_target_for_device(&dev);
                    self.refresh_target_comparisons();
                    self.busy = Some(BusyOperation::ReadingDeviceInfo);
                    self.progress = ProgressSnapshot::indeterminate(
                        ProgressStage::ReadingController,
                        "Reading controller firmware version…",
                    );
                    return vec![AppCommand::ReadDeviceFirmwareInfo {
                        device: dev.target(),
                    }];
                }
                Vec::new()
            }
            Err(CoreError::Cancelled) => {
                self.banner = Some(BannerMessage::info(
                    "Scan cancelled",
                    "Controller detection was cancelled.",
                ));
                Vec::new()
            }
            Err(err) => {
                self.enter_error(err, AppStep::DetectController);
                Vec::new()
            }
        }
    }

    fn handle_device_firmware_info_loaded(
        &mut self,
        path: &str,
        result: std::result::Result<DeviceInspection, CoreError>,
    ) -> Vec<AppCommand> {
        self.busy = None;
        self.progress = ProgressSnapshot::idle();

        match result {
            Ok(inspection) => {
                let fw_version_str = inspection.firmware_info.formatted_version();
                let mut loaded_device = None;
                if let Some(dev) = self.devices.iter_mut().find(|d| d.path == path) {
                    dev.apply_inspection(&inspection);
                    loaded_device = Some(dev.clone());
                }
                if self.selected_device_path.as_deref() == Some(path)
                    && let Some(dev) = loaded_device.as_ref()
                {
                    self.auto_match_target_for_device(dev);
                }
                self.refresh_target_comparisons();
                if let Some(warning) = loaded_device
                    .as_ref()
                    .and_then(|d| d.unsupported_board_warning.as_ref())
                {
                    self.banner = Some(BannerMessage::warning(
                        "Unsupported controller board",
                        warning.clone(),
                    ));
                } else {
                    self.banner = Some(BannerMessage::positive(
                        "Controller ready",
                        format!("Detected current firmware version {fw_version_str}."),
                    ));
                }
            }
            Err(CoreError::Cancelled) => {
                self.banner = Some(BannerMessage::info(
                    "Cancelled",
                    "Stopped reading controller firmware information.",
                ));
            }
            Err(err) => {
                self.enter_error(err, AppStep::DetectController);
            }
        }
        Vec::new()
    }

    fn handle_catalog_loaded(
        &mut self,
        result: std::result::Result<FirmwareCatalog, CoreError>,
    ) -> Vec<AppCommand> {
        self.busy = None;
        self.progress = ProgressSnapshot::idle();

        match result {
            Ok(catalog) => {
                self.catalog_checked = true;
                self.catalog_releases = catalog.releases.clone();

                for release in &catalog.releases {
                    if let Some(existing) = self
                        .targets
                        .iter_mut()
                        .find(|t| t.target_id.eq_ignore_ascii_case(&release.target_id))
                    {
                        existing.apply_release(release);
                    } else {
                        let info = find_known_target(&release.target_id);
                        let mut option = match info {
                            Some(known) => TargetOption::from_known(
                                known.target_id,
                                known.model_name,
                                known.board_revisions,
                                known.tested,
                            ),
                            None => TargetOption::from_known(
                                &release.target_id,
                                "DualSense",
                                &release.target_id,
                                false,
                            ),
                        };
                        option.apply_release(release);
                        self.targets.push(option);
                    }
                }
                self.refresh_target_comparisons();
                self.refresh_catalog_banner();
            }
            Err(CoreError::Cancelled) => {
                self.banner = Some(BannerMessage::info(
                    "Update check cancelled",
                    "Stopped checking Sony's firmware server.",
                ));
            }
            Err(err) => {
                let return_to = self.step;
                self.enter_error(err, return_to);
            }
        }
        Vec::new()
    }

    fn handle_local_file_inspected(
        &mut self,
        path: PathBuf,
        result: std::result::Result<LocalFirmwareInfo, CoreError>,
    ) -> Vec<AppCommand> {
        self.busy = None;
        self.progress = ProgressSnapshot::idle();

        match result {
            Ok(info) => {
                let summary = LocalFirmwareSummary::from_info(path, info);
                self.banner = Some(BannerMessage::positive(
                    "Firmware file ready",
                    format!(
                        "Loaded {} (version {}, {}).",
                        summary.file_name, summary.formatted_version, summary.size_label
                    ),
                ));
                self.local_firmware = Some(summary);
                self.warning_acknowledged = false;
            }
            Err(CoreError::Cancelled) => {
                self.banner = Some(BannerMessage::info(
                    "Cancelled",
                    "Stopped inspecting the selected firmware file.",
                ));
            }
            Err(err) => {
                self.local_firmware = None;
                let return_to = self.step;
                self.enter_error(err, return_to);
            }
        }
        Vec::new()
    }

    fn handle_update_finished(
        &mut self,
        result: std::result::Result<u16, CoreError>,
    ) -> Vec<AppCommand> {
        let was_flashing = matches!(self.busy, Some(BusyOperation::FlashingFirmware));
        self.busy = None;

        match result {
            Ok(version) => {
                if let Some(selected_path) = &self.selected_device_path
                    && let Some(dev) = self.devices.iter_mut().find(|d| d.path == *selected_path)
                {
                    dev.firmware_version = Some(version);
                    dev.formatted_firmware_version = format!("0x{version:04x}");
                }
                self.refresh_target_comparisons();
                self.completed_version = Some(version);
                self.warning_acknowledged = false;
                self.step = AppStep::Completed;
                self.progress = ProgressSnapshot::completed();
                self.banner = Some(BannerMessage::positive(
                    "Firmware update complete",
                    format!(
                        "Your DualSense controller is now updated to version 0x{version:04x}. You can unplug and use it normally."
                    ),
                ));
            }
            Err(CoreError::Cancelled) => {
                self.step = AppStep::ConfirmUpdate;
                self.warning_acknowledged = false;
                self.progress = ProgressSnapshot::idle();
                self.banner = Some(if was_flashing {
                    BannerMessage::warning(
                        "Update cancelled",
                        "The update was stopped. Reconnect your controller and check its firmware version.",
                    )
                } else {
                    BannerMessage::info(
                        "Download cancelled",
                        "The download was cancelled before anything was written to your controller.",
                    )
                });
            }
            Err(err) => {
                self.progress = ProgressSnapshot::idle();
                self.warning_acknowledged = false;
                self.enter_error(err, AppStep::ConfirmUpdate);
            }
        }
        Vec::new()
    }

    fn enter_error(&mut self, err: CoreError, return_step: AppStep) {
        let friendly = map_core_error(&err);
        self.previous_step = return_step;
        self.step = AppStep::Error;
        self.banner = Some(BannerMessage::error(
            friendly.error_title.clone(),
            friendly.error_message.clone(),
        ));
        self.error = Some(friendly);
    }

    fn selected_device(&self) -> Option<&DeviceSummary> {
        let path = self.selected_device_path.as_deref()?;
        self.devices.iter().find(|d| d.path == path)
    }

    fn selected_target(&self) -> Option<&TargetOption> {
        self.targets
            .iter()
            .find(|t| t.target_id.eq_ignore_ascii_case(&self.selected_target_id))
    }

    fn selected_release(&self) -> Option<&FirmwareRelease> {
        self.catalog_releases
            .iter()
            .find(|r| r.target_id.eq_ignore_ascii_case(&self.selected_target_id))
    }

    fn selected_device_board_supported(&self) -> bool {
        self.selected_device().is_some_and(|d| d.board_supported)
    }

    fn has_ready_firmware(&self) -> bool {
        match self.source_kind {
            FirmwareSourceKind::OfficialCatalog => {
                self.selected_device_board_supported() && self.selected_release().is_some()
            }
            FirmwareSourceKind::LocalFile => self.local_firmware.is_some(),
        }
    }

    fn auto_match_target_for_device(&mut self, dev: &DeviceSummary) {
        if let Some(target_id) = &dev.detected_target_id {
            self.selected_target_id = target_id.clone();
        } else if dev.is_edge {
            self.selected_target_id = "0044".to_string();
        } else if self.selected_target_id == "0044" {
            self.selected_target_id = "0004".to_string();
        }
    }

    fn refresh_target_comparisons(&mut self) {
        let current_version = self.selected_device().and_then(|d| d.firmware_version);
        for target in &mut self.targets {
            target.update_comparison(current_version);
        }
    }

    fn refresh_catalog_banner(&mut self) {
        if !self.catalog_checked {
            return;
        }
        let Some(target) = self.selected_target() else {
            return;
        };
        if let Some(true) = target.is_up_to_date {
            self.banner = Some(BannerMessage::positive(
                "Controller is up to date",
                format!(
                    "Your controller is already running version {} (latest for {}).",
                    target.latest_version_label, target.label
                ),
            ));
        } else if let Some(latest) = &target.latest_version {
            self.banner = Some(BannerMessage::info(
                "Update available",
                format!(
                    "Latest official firmware for {} is 0x{latest:04x}.",
                    target.label
                ),
            ));
        }
    }

    fn build_update_summary(&self) -> Option<UpdateSummary> {
        let device = self.selected_device()?;
        let current_version_label = device.formatted_firmware_version.clone();

        match self.source_kind {
            FirmwareSourceKind::OfficialCatalog => {
                if !device.board_supported {
                    return None;
                }
                let target = self.selected_target()?;
                let latest = target.latest_version?;
                let is_same_or_older_version = device
                    .firmware_version
                    .is_some_and(|current| current >= latest);
                Some(UpdateSummary {
                    controller_label: device.label.clone(),
                    current_version_label,
                    target_version_label: format!("0x{latest:04x}"),
                    source_description: format!("Official Sony update — {}", target.label),
                    is_untested_hardware: !target.tested,
                    is_same_or_older_version,
                })
            }
            FirmwareSourceKind::LocalFile => {
                let local = self.local_firmware.as_ref()?;
                let is_same_or_older_version = device
                    .firmware_version
                    .is_some_and(|current| current >= local.version);
                Some(UpdateSummary {
                    controller_label: device.label.clone(),
                    current_version_label,
                    target_version_label: local.formatted_version.clone(),
                    source_description: format!("Local file — {}", local.file_name),
                    is_untested_hardware: device.is_edge,
                    is_same_or_older_version,
                })
            }
        }
    }
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn compute_percentage(current: usize, total: usize) -> u8 {
    if total == 0 {
        return 100;
    }
    let ratio = (current as f64 / total as f64) * 100.0;
    ratio.round().clamp(0.0, 100.0) as u8
}

/// Formats a byte count into a human-friendly size string (`B`, `KB`, `MB`).
pub fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use dualsense_core::{DEFAULT_PID, DEFAULT_VID, StartUpdateError, UpdateFailure};

    use super::*;

    fn sample_hid_device(path: &str, pid: u16, serial: &str) -> HidDeviceInfo {
        HidDeviceInfo {
            path: path.to_string(),
            vendor_id: DEFAULT_VID,
            product_id: pid,
            interface_number: 3,
            usage_page: 1,
            usage: 5,
            product_string: if pid == DUALSENSE_EDGE_PID {
                "DualSense Edge Wireless Controller".to_string()
            } else {
                "DualSense Wireless Controller".to_string()
            },
            serial_number: serial.to_string(),
        }
    }

    fn sample_firmware_info_with_hw(version: u16, fw_series: u16, hw_info: u32) -> FirmwareInfo {
        let mut raw = vec![0u8; 64];
        raw[0] = dualsense_core::REPORT_ID_FIRMWARE_INFO;
        raw[1..12].copy_from_slice(b"Oct  1 2024");
        raw[12..20].copy_from_slice(b"12:34:56");
        raw[22..24].copy_from_slice(&fw_series.to_le_bytes());
        raw[24..28].copy_from_slice(&hw_info.to_le_bytes());
        raw[44..46].copy_from_slice(&version.to_le_bytes());
        FirmwareInfo::from_report(raw).expect("valid firmware info")
    }

    fn sample_inspection(
        version: u16,
        pid: u16,
        fw_series: u16,
        hw_info: u32,
        serial: Option<&str>,
    ) -> DeviceInspection {
        let fw_info = sample_firmware_info_with_hw(version, fw_series, hw_info);
        let hardware = resolve_controller_target(&fw_info, pid);
        let serial_number = serial.map(ToString::to_string);
        let color_name = serial
            .and_then(dualsense_core::decode_ds5_color)
            .map(ToString::to_string);
        DeviceInspection {
            firmware_info: fw_info,
            hardware,
            serial_number,
            color_name,
        }
    }

    fn sample_bdm020_inspection(version: u16) -> DeviceInspection {
        sample_inspection(version, DEFAULT_PID, 0x0004, 0x0000_0411, None)
    }

    fn sample_edge_inspection(version: u16) -> DeviceInspection {
        sample_inspection(version, DUALSENSE_EDGE_PID, 0x0044, 0x0000_0400, None)
    }

    fn sample_catalog() -> FirmwareCatalog {
        FirmwareCatalog::from_json(
            r#"{"ApplicationLatestVersion":"2.2.1.2","FwUpdate0004LatestVersion":"0x0630","FwUpdate000BLatestVersion":"0x0630","FwUpdate000ELatestVersion":"0x0641","FwUpdate0044LatestVersion":"0x0217"}"#,
        )
        .expect("valid catalog")
    }

    #[test]
    fn initial_state_has_known_targets_and_proper_gating() {
        let state = AppState::new();
        let snap = state.snapshot();

        assert_eq!(snap.step, AppStep::DetectController);
        assert_eq!(snap.step_index, 0);
        assert_eq!(snap.targets.len(), KNOWN_TARGETS.len());
        assert_eq!(snap.selected_target_id, "0004");
        assert!(!snap.is_busy);
        assert!(snap.can_scan);
        assert!(!snap.can_continue_to_firmware);
        assert!(!snap.can_select_target);
        assert!(!snap.can_check_updates);
        assert!(!snap.can_select_file);
        assert!(!snap.can_review_update);
        assert!(!snap.can_start_update);
        assert!(!snap.can_cancel);
        assert!(!snap.can_go_back);
        assert!(!snap.board_supported);
        assert!(snap.unsupported_board_warning.is_none());
        assert_eq!(
            snap.banner.as_ref().map(|b| b.severity),
            Some(BannerSeverity::Info)
        );
    }

    #[test]
    fn scan_select_and_auto_edge_target_selection_flow() {
        let mut state = AppState::new();
        let cmds = state.update(AppEvent::ScanDevices);
        assert_eq!(cmds, vec![AppCommand::ScanDevices]);
        let busy_snap = state.snapshot();
        assert!(busy_snap.is_busy);
        assert!(!busy_snap.can_scan);
        assert!(busy_snap.can_cancel);
        assert!(busy_snap.progress.indeterminate);

        // Empty scan shows helpful USB cable warning
        let empty_cmds = state.update(AppEvent::DevicesScanned(Ok(Vec::new())));
        assert!(empty_cmds.is_empty());
        let empty_snap = state.snapshot();
        assert!(!empty_snap.is_busy);
        assert!(!empty_snap.can_continue_to_firmware);
        assert_eq!(
            empty_snap.banner.as_ref().map(|b| b.severity),
            Some(BannerSeverity::Warning)
        );

        // Scan finding two devices (standard + Edge) auto-probes first device
        let d1 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
        let d2 = sample_hid_device("/dev/hidraw1", DUALSENSE_EDGE_PID, "EDGE-1");
        let cmds = state.update(AppEvent::DevicesScanned(Ok(vec![d1.clone(), d2.clone()])));
        assert_eq!(
            cmds,
            vec![AppCommand::ReadDeviceFirmwareInfo {
                device: DeviceTarget {
                    path: "/dev/hidraw0".to_string(),
                    vendor_id: DEFAULT_VID,
                    product_id: DEFAULT_PID,
                }
            }]
        );

        // Firmware info arrives
        state.update(AppEvent::DeviceFirmwareInfoLoaded {
            path: "/dev/hidraw0".to_string(),
            result: Ok(sample_bdm020_inspection(0x0520)),
        });
        let snap = state.snapshot();
        assert!(snap.can_continue_to_firmware);
        assert!(snap.board_supported);
        assert!(snap.can_select_target);
        assert_eq!(snap.board_model, "BDM-020");
        assert_eq!(snap.detected_target_id.as_deref(), Some("0004"));
        assert_eq!(
            snap.selected_device
                .as_ref()
                .map(|d| d.formatted_firmware_version.as_str()),
            Some("0x0520")
        );
        assert_eq!(
            snap.banner.as_ref().map(|b| b.severity),
            Some(BannerSeverity::Positive)
        );

        // Selecting Edge controller auto-selects target 0044 and probes its info
        let cmds = state.update(AppEvent::SelectDevice {
            path: "/dev/hidraw1".to_string(),
        });
        assert_eq!(cmds.len(), 1);
        assert_eq!(state.snapshot().selected_target_id, "0044");
        state.update(AppEvent::DeviceFirmwareInfoLoaded {
            path: "/dev/hidraw1".to_string(),
            result: Ok(sample_edge_inspection(0x0217)),
        });

        // Selecting back to standard controller switches away from 0044 without re-probing
        let cmds = state.update(AppEvent::SelectDevice {
            path: "/dev/hidraw0".to_string(),
        });
        assert!(cmds.is_empty());
        assert_eq!(state.snapshot().selected_target_id, "0004");
    }

    #[test]
    fn guided_catalog_confirm_progress_and_completion_flow() {
        let mut state = AppState::new();
        let d1 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
        state.update(AppEvent::DevicesScanned(Ok(vec![d1])));
        state.update(AppEvent::DeviceFirmwareInfoLoaded {
            path: "/dev/hidraw0".to_string(),
            result: Ok(sample_bdm020_inspection(0x0520)),
        });
        state.update(AppEvent::ContinueToFirmwareStep);

        let snap = state.snapshot();
        assert_eq!(snap.step, AppStep::SelectFirmware);
        assert!(snap.can_check_updates);
        assert!(snap.can_select_file);
        assert!(!snap.can_review_update);
        assert!(snap.can_go_back);

        // Fetch catalog
        let cmds = state.update(AppEvent::CheckForUpdates);
        assert_eq!(cmds, vec![AppCommand::FetchCatalog]);
        assert!(state.snapshot().can_cancel);
        assert!(!state.snapshot().can_check_updates);

        state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));
        let snap = state.snapshot();
        assert!(snap.catalog_checked);
        assert!(snap.can_review_update);
        assert_eq!(
            snap.selected_target.as_ref().and_then(|t| t.latest_version),
            Some(0x0630)
        );
        assert_eq!(
            snap.selected_target.as_ref().and_then(|t| t.is_up_to_date),
            Some(false)
        );

        // Continue to ConfirmUpdate
        state.update(AppEvent::ContinueToConfirmStep);
        let confirm_snap = state.snapshot();
        assert_eq!(confirm_snap.step, AppStep::ConfirmUpdate);
        assert!(!confirm_snap.can_start_update);
        assert!(
            confirm_snap
                .irreversible_warning
                .contains("cannot be downgraded")
        );
        let summary = confirm_snap.update_summary.expect("summary present");
        assert_eq!(summary.current_version_label, "0x0520");
        assert_eq!(summary.target_version_label, "0x0630");
        assert!(!summary.is_same_or_older_version);

        // Acknowledge warning and start update
        state.update(AppEvent::SetWarningAcknowledged(true));
        assert!(state.snapshot().can_start_update);

        let cmds = state.update(AppEvent::StartUpdate);
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0], AppCommand::DownloadAndInstall { .. }));
        let dl_snap = state.snapshot();
        assert_eq!(dl_snap.step, AppStep::Updating);
        assert!(dl_snap.is_busy);
        assert!(dl_snap.can_cancel);
        assert!(!dl_snap.can_start_update);

        // Download progress formatting
        state.update(AppEvent::DownloadProgressUpdated(DownloadProgress {
            downloaded_bytes: 45,
            total_bytes: Some(100),
        }));
        let dl_prog = state.snapshot();
        assert_eq!(dl_prog.progress.status_text, "Downloading firmware… (45%)");
        assert_eq!(dl_prog.progress.percentage, Some(45));
        assert!(dl_prog.can_cancel);

        // Indeterminate download progress formatting
        state.update(AppEvent::DownloadProgressUpdated(DownloadProgress {
            downloaded_bytes: 2048,
            total_bytes: None,
        }));
        let dl_indet = state.snapshot();
        assert_eq!(dl_indet.progress.status_text, "Downloading firmware…");
        assert!(dl_indet.progress.indeterminate);
        assert_eq!(dl_indet.progress.detail_text, "2.0 KB downloaded");

        // Update transitions to flashing -> can_cancel becomes false!
        state.update(AppEvent::UpdateProgressUpdated(UpdateProgress {
            phase: UpdatePhase::Starting,
            current_bytes: 0,
            total_bytes: 65536,
            current_block: 0,
            total_blocks: 2,
            percentage: 0.0,
        }));
        assert!(!state.snapshot().can_cancel);
        assert_eq!(
            state.snapshot().progress.status_text,
            "Preparing controller for update…"
        );

        state.update(AppEvent::UpdateProgressUpdated(UpdateProgress {
            phase: UpdatePhase::Writing,
            current_bytes: 40632,
            total_bytes: 65536,
            current_block: 2,
            total_blocks: 2,
            percentage: 62.0,
        }));
        assert_eq!(
            state.snapshot().progress.status_text,
            "Installing firmware… (62%)"
        );
        assert_eq!(state.snapshot().progress.percentage, Some(62));

        state.update(AppEvent::UpdateProgressUpdated(UpdateProgress {
            phase: UpdatePhase::Verifying,
            current_bytes: 65536,
            total_bytes: 65536,
            current_block: 2,
            total_blocks: 2,
            percentage: 100.0,
        }));
        assert_eq!(
            state.snapshot().progress.status_text,
            "Verifying installed firmware…"
        );

        state.update(AppEvent::UpdateProgressUpdated(UpdateProgress {
            phase: UpdatePhase::Finalizing,
            current_bytes: 65536,
            total_bytes: 65536,
            current_block: 2,
            total_blocks: 2,
            percentage: 100.0,
        }));
        assert_eq!(
            state.snapshot().progress.status_text,
            "Finalizing firmware update…"
        );

        // Finish update
        state.update(AppEvent::UpdateFinished(Ok(0x0630)));
        let done_snap = state.snapshot();
        assert_eq!(done_snap.step, AppStep::Completed);
        assert!(!done_snap.is_busy);
        assert_eq!(done_snap.completed_version.as_deref(), Some("0x0630"));
        assert_eq!(done_snap.progress.percentage, Some(100));
        assert_eq!(
            done_snap.selected_target.and_then(|t| t.is_up_to_date),
            Some(true)
        );
        assert_eq!(
            done_snap.banner.as_ref().map(|b| b.severity),
            Some(BannerSeverity::Positive)
        );

        // ResetFlow returns cleanly to DetectController
        state.update(AppEvent::ResetFlow);
        assert_eq!(state.snapshot().step, AppStep::DetectController);
    }

    #[test]
    fn local_file_flow_cancellation_and_friendly_error_recovery() {
        let mut state = AppState::new();
        let d1 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
        state.update(AppEvent::DevicesScanned(Ok(vec![d1])));
        state.update(AppEvent::DeviceFirmwareInfoLoaded {
            path: "/dev/hidraw0".to_string(),
            result: Ok(sample_bdm020_inspection(0x0630)),
        });
        state.update(AppEvent::ContinueToFirmwareStep);

        let path = PathBuf::from("/tmp/fw.bin");
        let cmds = state.update(AppEvent::SelectLocalFile { path: path.clone() });
        assert_eq!(
            cmds,
            vec![AppCommand::InspectLocalFile { path: path.clone() }]
        );

        state.update(AppEvent::LocalFileInspected {
            path: path.clone(),
            result: Ok(LocalFirmwareInfo {
                version: 0x0630,
                size_bytes: 1_572_864,
            }),
        });
        let snap = state.snapshot();
        assert!(snap.can_review_update);
        assert_eq!(
            snap.local_firmware.as_ref().map(|f| f.size_label.as_str()),
            Some("1.50 MB")
        );

        // Confirm & detect same-or-older version warning flag
        state.update(AppEvent::ContinueToConfirmStep);
        assert!(
            state
                .snapshot()
                .update_summary
                .expect("summary")
                .is_same_or_older_version
        );

        // GoBack and back forward
        state.update(AppEvent::GoBack);
        assert_eq!(state.snapshot().step, AppStep::SelectFirmware);
        state.update(AppEvent::ContinueToConfirmStep);
        state.update(AppEvent::SetWarningAcknowledged(true));

        let cmds = state.update(AppEvent::StartUpdate);
        assert!(matches!(cmds[0], AppCommand::InstallLocalFirmware { .. }));
        // Local file flashing cannot be cancelled once started
        assert!(!state.snapshot().can_cancel);

        // Simulate failure from controller
        state.update(AppEvent::UpdateFinished(Err(CoreError::UpdateFailed(
            UpdateFailure::StartUpdate(StartUpdateError::HeaderVersionCheckError),
        ))));
        let err_snap = state.snapshot();
        assert_eq!(err_snap.step, AppStep::Error);
        assert!(err_snap.error_title.contains("already has this"));
        assert!(!err_snap.recovery_hint.is_empty());

        // DismissError returns to ConfirmUpdate with warning unchecked
        state.update(AppEvent::DismissError);
        let recovered = state.snapshot();
        assert_eq!(recovered.step, AppStep::ConfirmUpdate);
        assert!(!recovered.warning_acknowledged);
        assert!(recovered.error.is_none());
    }

    #[test]
    fn download_cancellation_returns_to_confirm_step_cleanly() {
        let mut state = AppState::new();
        let d1 = sample_hid_device("/dev/hidraw0", DEFAULT_PID, "STD-1");
        state.update(AppEvent::DevicesScanned(Ok(vec![d1])));
        state.update(AppEvent::DeviceFirmwareInfoLoaded {
            path: "/dev/hidraw0".to_string(),
            result: Ok(sample_bdm020_inspection(0x0520)),
        });
        state.update(AppEvent::ContinueToFirmwareStep);
        state.update(AppEvent::CatalogLoaded(Ok(sample_catalog())));
        state.update(AppEvent::ContinueToConfirmStep);
        state.update(AppEvent::SetWarningAcknowledged(true));
        state.update(AppEvent::StartUpdate);

        let cancel_cmds = state.update(AppEvent::RequestCancel);
        assert_eq!(cancel_cmds, vec![AppCommand::CancelActiveOperation]);

        state.update(AppEvent::UpdateFinished(Err(CoreError::Cancelled)));
        let snap = state.snapshot();
        assert_eq!(snap.step, AppStep::ConfirmUpdate);
        assert!(!snap.warning_acknowledged);
        assert_eq!(
            snap.banner.as_ref().map(|b| b.severity),
            Some(BannerSeverity::Info)
        );
        assert!(
            snap.banner
                .as_ref()
                .expect("banner")
                .text
                .contains("before anything was written")
        );
    }

    #[test]
    fn handles_extra_catalog_target_and_busy_guards() {
        let mut state = AppState::new();
        // Select non-existent target does nothing
        state.update(AppEvent::SelectTarget {
            target_id: "9999".to_string(),
        });
        assert_eq!(state.snapshot().selected_target_id, "0004");

        // Extra target in catalog is appended cleanly
        let catalog = FirmwareCatalog::from_json(
            r#"{"FwUpdate0004LatestVersion":"0x0630","FwUpdate0099LatestVersion":"0x0100"}"#,
        )
        .expect("catalog");
        state.update(AppEvent::CatalogLoaded(Ok(catalog)));
        assert!(
            state
                .snapshot()
                .targets
                .iter()
                .any(|t| t.target_id == "0099" && t.latest_version == Some(0x0100))
        );

        // Busy guard ignores concurrent user commands except RequestCancel
        state.update(AppEvent::ScanDevices);
        assert!(state.update(AppEvent::ScanDevices).is_empty());
        assert!(state.update(AppEvent::GoBack).is_empty());
        assert!(state.update(AppEvent::ResetFlow).is_empty());
        assert!(state.update(AppEvent::DismissError).is_empty());
        assert!(state.update(AppEvent::StartUpdate).is_empty());
    }

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
        let step2 = state.snapshot();
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

        // Local file remains usable on unsupported boards
        let path = PathBuf::from("/tmp/custom.bin");
        let cmds = state.update(AppEvent::SelectLocalFile { path: path.clone() });
        assert_eq!(
            cmds,
            vec![AppCommand::InspectLocalFile { path: path.clone() }]
        );
        state.update(AppEvent::LocalFileInspected {
            path,
            result: Ok(LocalFirmwareInfo {
                version: 0x0250,
                size_bytes: 65536,
            }),
        });
        assert!(state.snapshot().can_review_update);

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
}
