use dualsense_core::{FirmwareRelease, KNOWN_TARGETS};

use crate::error_map::{FriendlyError, map_core_error};

use super::progress::ProgressSnapshot;
use super::types::{
    AppCommand, AppEvent, AppStep, BannerMessage, BusyOperation, DeviceSummary, FirmwareSourceKind,
    LocalFirmwareSummary, TargetOption,
};

/// Pure, deterministic Elm-style state machine for the DualSense firmware updater UI.
#[derive(Debug, Clone)]
pub struct AppState {
    pub(super) step: AppStep,
    pub(super) previous_step: AppStep,
    pub(super) devices: Vec<DeviceSummary>,
    pub(super) selected_device_path: Option<String>,
    pub(super) source_kind: FirmwareSourceKind,
    pub(super) targets: Vec<TargetOption>,
    pub(super) selected_target_id: String,
    pub(super) catalog_releases: Vec<FirmwareRelease>,
    pub(super) catalog_checked: bool,
    pub(super) local_firmware: Option<LocalFirmwareSummary>,
    pub(super) warning_acknowledged: bool,
    pub(super) completed_version: Option<u16>,
    pub(super) banner: Option<BannerMessage>,
    pub(super) progress: ProgressSnapshot,
    pub(super) error: Option<FriendlyError>,
    pub(super) busy: Option<BusyOperation>,
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

    pub(super) fn enter_error(&mut self, err: dualsense_core::CoreError, return_step: AppStep) {
        let friendly = map_core_error(&err);
        self.previous_step = return_step;
        self.step = AppStep::Error;
        self.banner = Some(BannerMessage::error(
            friendly.error_title.clone(),
            friendly.error_message.clone(),
        ));
        self.error = Some(friendly);
    }

    pub(super) fn selected_device(&self) -> Option<&DeviceSummary> {
        let path = self.selected_device_path.as_deref()?;
        self.devices.iter().find(|d| d.path == path)
    }

    pub(super) fn selected_target(&self) -> Option<&TargetOption> {
        self.targets
            .iter()
            .find(|t| t.target_id.eq_ignore_ascii_case(&self.selected_target_id))
    }

    pub(super) fn selected_release(&self) -> Option<&FirmwareRelease> {
        self.catalog_releases
            .iter()
            .find(|r| r.target_id.eq_ignore_ascii_case(&self.selected_target_id))
    }

    pub(super) fn selected_device_board_supported(&self) -> bool {
        self.selected_device().is_some_and(|d| d.board_supported)
    }

    pub(super) fn has_ready_firmware(&self) -> bool {
        match self.source_kind {
            FirmwareSourceKind::OfficialCatalog => {
                self.selected_device_board_supported() && self.selected_release().is_some()
            }
            FirmwareSourceKind::LocalFile => self.local_firmware.is_some(),
        }
    }

    pub(super) fn auto_match_target_for_device(&mut self, dev: &DeviceSummary) {
        if let Some(target_id) = &dev.detected_target_id {
            self.selected_target_id = target_id.clone();
        } else if dev.is_edge {
            self.selected_target_id = "0044".to_string();
        } else if self.selected_target_id == "0044" {
            self.selected_target_id = "0004".to_string();
        }
    }

    pub(super) fn refresh_target_comparisons(&mut self) {
        let current_version = self.selected_device().and_then(|d| d.firmware_version);
        for target in &mut self.targets {
            target.update_comparison(current_version);
        }
    }

    pub(super) fn refresh_catalog_banner(&mut self) {
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
}
