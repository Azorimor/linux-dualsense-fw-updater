// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeSet;

use dualsense_core::{
    FirmwareRelease, HidDeviceInfo, KNOWN_TARGETS, LocalImageCompatibility,
    filter_bluetooth_devices, filter_usb_devices,
};

use crate::error_map::{FriendlyError, map_core_error};
use crate::flash_report::{FlashReport, PendingFlashAttempt, SystemEnvironmentInfo};

use super::progress::ProgressSnapshot;
use super::types::{
    AppCommand, AppEvent, AppStep, BannerMessage, BusyOperation, CachedFirmwareMeta, DeviceSummary,
    FirmwareSourceKind, LocalFirmwareInfo, LocalFirmwareSummary, TargetOption,
};

/// Pure, deterministic Elm-style state machine for the DualSense firmware updater UI.
#[derive(Debug, Clone)]
pub struct AppState {
    pub(super) step: AppStep,
    pub(super) previous_step: AppStep,
    pub(super) devices: Vec<DeviceSummary>,
    pub(super) bluetooth_devices: Vec<HidDeviceInfo>,
    pub(super) pending_device_inspections: BTreeSet<String>,
    pub(super) selected_device_path: Option<String>,
    pub(super) source_kind: FirmwareSourceKind,
    pub(super) targets: Vec<TargetOption>,
    pub(super) selected_target_id: String,
    pub(super) catalog_releases: Vec<FirmwareRelease>,
    pub(super) catalog_checked: bool,
    pub(super) catalog_checking: bool,
    pub(super) local_firmware_info: Option<LocalFirmwareInfo>,
    pub(super) local_firmware: Option<LocalFirmwareSummary>,
    pub(super) warning_acknowledged: bool,
    pub(super) completed_version: Option<u16>,
    pub(super) post_flash_verified: bool,
    pub(super) completed_verification_note: Option<String>,
    pub(super) cached_firmware: Option<CachedFirmwareMeta>,
    pub(super) banner: Option<BannerMessage>,
    pub(super) progress: ProgressSnapshot,
    pub(super) error: Option<FriendlyError>,
    pub(super) busy: Option<BusyOperation>,
    pub(super) system_info: SystemEnvironmentInfo,
    pub(super) pending_flash_attempt: Option<PendingFlashAttempt>,
    pub(super) flash_report: Option<FlashReport>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Creates the initial application state with known DualSense hardware targets pre-populated.
    pub fn new() -> Self {
        Self::with_system_info(SystemEnvironmentInfo::detect())
    }

    /// Creates initial application state with an explicit [`SystemEnvironmentInfo`] (useful for deterministic tests).
    pub fn with_system_info(system_info: SystemEnvironmentInfo) -> Self {
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
            bluetooth_devices: Vec::new(),
            pending_device_inspections: BTreeSet::new(),
            selected_device_path: None,
            source_kind: FirmwareSourceKind::OfficialCatalog,
            targets,
            selected_target_id: "0004".to_string(),
            catalog_releases: Vec::new(),
            catalog_checked: false,
            catalog_checking: false,
            local_firmware_info: None,
            local_firmware: None,
            warning_acknowledged: false,
            completed_version: None,
            post_flash_verified: false,
            completed_verification_note: None,
            cached_firmware: None,
            banner: Some(BannerMessage::info(
                "Connect your DualSense controller",
                "Plug your controller into your PC with a USB cable, then click Scan for Controllers.",
            )),
            progress: ProgressSnapshot::idle(),
            error: None,
            busy: None,
            system_info,
            pending_flash_attempt: None,
            flash_report: None,
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
                if kind == FirmwareSourceKind::LocalFile {
                    self.refresh_local_firmware_banner();
                } else {
                    self.refresh_catalog_banner();
                }
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
                    && self.selected_device().is_some()
                {
                    self.step = AppStep::SelectFirmware;
                    self.warning_acknowledged = false;
                }
                Vec::new()
            }
            AppEvent::ContinueToConfirmStep => self.handle_continue_to_confirm(),
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
            AppEvent::SaveCachedFirmware { path } => self.handle_save_cached_firmware(path),
            AppEvent::RequestCancel => self.handle_request_cancel(),
            AppEvent::DismissError => {
                if self.step == AppStep::Error && self.busy.is_none() {
                    self.error = None;
                    self.pending_flash_attempt = None;
                    self.flash_report = None;
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
                self.post_flash_verified = false;
                self.completed_verification_note = None;
                self.pending_flash_attempt = None;
                self.flash_report = None;
                self.warning_acknowledged = false;
                self.progress = ProgressSnapshot::idle();
                self.banner = None;
                Vec::new()
            }
            AppEvent::DevicesScanned(result) => self.handle_devices_scanned(result),
            AppEvent::DevicesPollUpdated(devices) => self.handle_devices_poll_updated(devices),
            AppEvent::DeviceFirmwareInfoLoaded { path, result } => {
                self.handle_device_firmware_info_loaded(&path, result)
            }
            AppEvent::CatalogLoaded(result) => self.handle_catalog_loaded(result),
            AppEvent::LocalFileInspected { path, result } => {
                self.handle_local_file_inspected(path, result)
            }
            AppEvent::OfficialFirmwareCached { target_id, version } => {
                self.cached_firmware = Some(CachedFirmwareMeta { target_id, version });
                Vec::new()
            }
            AppEvent::CachedFirmwareSaved { path, result } => {
                self.handle_cached_firmware_saved(path, result)
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

    /// Returns `true` when firmware is actively being flashed to a controller.
    pub(crate) fn is_flashing(&self) -> bool {
        matches!(self.busy, Some(BusyOperation::FlashingFirmware))
    }

    /// Returns `true` if a background poll differs from known USB/Bluetooth devices or the active device vanished.
    pub(crate) fn poll_devices_changed(&self, polled: &[HidDeviceInfo]) -> bool {
        let usb_polled = filter_usb_devices(polled);
        let bt_polled = filter_bluetooth_devices(polled);

        if self
            .selected_device_path
            .as_deref()
            .is_some_and(|sel| !usb_polled.iter().any(|d| d.path == sel))
        {
            return true;
        }

        let mut current_usb: Vec<(&str, u16, u16)> = self
            .devices
            .iter()
            .map(|d| (d.path.as_str(), d.vendor_id, d.product_id))
            .collect();
        current_usb.sort_unstable();

        let mut next_usb: Vec<(&str, u16, u16)> = usb_polled
            .iter()
            .map(|d| (d.path.as_str(), d.vendor_id, d.product_id))
            .collect();
        next_usb.sort_unstable();

        if current_usb != next_usb {
            return true;
        }

        let mut current_bt: Vec<(&str, u16, u16)> = self
            .bluetooth_devices
            .iter()
            .map(|d| (d.path.as_str(), d.vendor_id, d.product_id))
            .collect();
        current_bt.sort_unstable();

        let mut next_bt: Vec<(&str, u16, u16)> = bt_polled
            .iter()
            .map(|d| (d.path.as_str(), d.vendor_id, d.product_id))
            .collect();
        next_bt.sort_unstable();

        current_bt != next_bt
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
            FirmwareSourceKind::LocalFile => self
                .local_firmware
                .as_ref()
                .is_some_and(|local| local.compatibility != LocalImageCompatibility::Incompatible),
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

    pub(super) fn revalidate_local_firmware(&mut self) {
        let Some(info) = self.local_firmware_info.as_ref() else {
            return;
        };
        let selected = self.selected_device();
        let (compatibility, warning, error) =
            LocalFirmwareSummary::evaluate_for_device(info, selected);
        if let Some(summary) = &mut self.local_firmware {
            summary.compatibility = compatibility;
            summary.warning = warning;
            summary.error = error;
        }
    }

    pub(super) fn selected_device_battery_warning(&self) -> Option<String> {
        let status = self.selected_device()?.battery_status?;
        if status.is_critically_low() {
            Some(format!(
                "Controller battery is critically low ({}) and not charging. Charge it or use a powered USB port before flashing.",
                status.formatted_summary()
            ))
        } else if status.is_low() {
            Some(format!(
                "Controller battery is low ({}) and not charging. Keep the USB cable firmly connected during the update.",
                status.formatted_summary()
            ))
        } else {
            None
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

    pub(super) fn refresh_local_firmware_banner(&mut self) {
        let Some(summary) = &self.local_firmware else {
            return;
        };
        self.banner = Some(match summary.compatibility {
            LocalImageCompatibility::Compatible => BannerMessage::positive(
                "Firmware file ready",
                format!(
                    "Loaded {} (version {}, {}).",
                    summary.file_name, summary.formatted_version, summary.size_label
                ),
            ),
            LocalImageCompatibility::Warning => BannerMessage::warning(
                "Firmware file loaded with warning",
                summary
                    .warning
                    .clone()
                    .unwrap_or_else(|| "Target verification warning.".to_string()),
            ),
            LocalImageCompatibility::Incompatible => BannerMessage::error(
                "Incompatible firmware file",
                summary.error.clone().unwrap_or_else(|| {
                    "This firmware file is not compatible with the selected controller.".to_string()
                }),
            ),
        });
    }
}
