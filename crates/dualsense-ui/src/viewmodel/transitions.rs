// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use dualsense_core::{
    CoreError, DownloadProgress, FirmwareCatalog, HidDeviceInfo, PostFlashVerificationStatus,
    filter_bluetooth_devices, filter_usb_devices, find_known_target,
};

use crate::error_map::map_core_error;
use crate::flash_report::{FlashReport, PendingFlashAttempt};

use super::progress::{ProgressSnapshot, ProgressStage};
use super::state::AppState;
use super::types::{
    AppCommand, AppStep, BannerMessage, BusyOperation, CachedFirmwareMeta, DeviceInspection,
    DeviceSummary, FirmwareSourceKind, LocalFirmwareInfo, LocalFirmwareSummary, TargetOption,
    UpdateOutcome,
};

impl AppState {
    pub(super) fn handle_scan_devices(&mut self) -> Vec<AppCommand> {
        if self.busy.is_some() || self.step == AppStep::Updating {
            return Vec::new();
        }
        self.error = None;
        self.completed_version = None;
        self.post_flash_verified = false;
        self.completed_verification_note = None;
        self.pending_flash_attempt = None;
        self.flash_report = None;
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

    pub(super) fn handle_select_device(&mut self, path: &str) -> Vec<AppCommand> {
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
        self.revalidate_local_firmware();

        if dev.firmware_version.is_none() {
            self.busy = Some(BusyOperation::ReadingDeviceInfo);
            self.progress = ProgressSnapshot::indeterminate(
                ProgressStage::ReadingController,
                "Reading controller firmware version…",
            );
            if self.pending_device_inspections.insert(dev.path.clone()) {
                return vec![AppCommand::ReadDeviceFirmwareInfo {
                    device: dev.target(),
                }];
            }
            return Vec::new();
        }

        if self.source_kind == FirmwareSourceKind::LocalFile
            && self.local_firmware.is_some()
            && self.step != AppStep::DetectController
        {
            self.refresh_local_firmware_banner();
        } else if let Some(warning) = &dev.unsupported_board_warning {
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

        if dev.board_supported && !self.catalog_checked && !self.catalog_checking {
            self.catalog_checking = true;
            return vec![AppCommand::FetchCatalog];
        }

        Vec::new()
    }

    pub(super) fn handle_refresh_device_info(&mut self) -> Vec<AppCommand> {
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
        self.pending_device_inspections.insert(dev.path.clone());
        vec![AppCommand::ReadDeviceFirmwareInfo {
            device: dev.target(),
        }]
    }

    pub(super) fn handle_check_for_updates(&mut self) -> Vec<AppCommand> {
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
        if self.catalog_checking {
            return Vec::new();
        }
        self.catalog_checking = true;
        vec![AppCommand::FetchCatalog]
    }

    pub(super) fn handle_select_local_file(&mut self, path: PathBuf) -> Vec<AppCommand> {
        if self.busy.is_some()
            || self.selected_device().is_none()
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

    pub(super) fn handle_continue_to_confirm(&mut self) -> Vec<AppCommand> {
        if self.busy.is_none()
            && self.step == AppStep::SelectFirmware
            && self.selected_device().is_some()
            && self.has_ready_firmware()
        {
            self.step = AppStep::ConfirmUpdate;
            self.warning_acknowledged = false;
            if let Some(battery_warn) = self.selected_device_battery_warning() {
                self.banner = Some(BannerMessage::warning(
                    "Low controller battery",
                    battery_warn,
                ));
            } else if self.source_kind == FirmwareSourceKind::LocalFile
                && let Some(local_warn) = self
                    .local_firmware
                    .as_ref()
                    .and_then(|l| l.warning.as_ref())
            {
                self.banner = Some(BannerMessage::warning(
                    "Local firmware warning",
                    local_warn.clone(),
                ));
            }
        }
        Vec::new()
    }

    pub(super) fn handle_start_update(&mut self) -> Vec<AppCommand> {
        if self.busy.is_some()
            || self.step != AppStep::ConfirmUpdate
            || !self.warning_acknowledged
            || !self.has_ready_firmware()
        {
            return Vec::new();
        }
        let Some(selected_summary) = self.selected_device().cloned() else {
            return Vec::new();
        };
        let device = selected_summary.target();

        match self.source_kind {
            FirmwareSourceKind::OfficialCatalog => {
                let Some(release) = self.selected_release().cloned() else {
                    return Vec::new();
                };
                let is_untested = release.target_info.is_none_or(|info| !info.tested);
                self.pending_flash_attempt = Some(PendingFlashAttempt::from_device(
                    &selected_summary,
                    FirmwareSourceKind::OfficialCatalog,
                    Some(release.target_id.clone()),
                    format!("0x{:04x}", release.version),
                    is_untested,
                ));
                self.flash_report = None;
                self.post_flash_verified = false;
                self.completed_verification_note = None;
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
                self.pending_flash_attempt = Some(PendingFlashAttempt::from_device(
                    &selected_summary,
                    FirmwareSourceKind::LocalFile,
                    None,
                    local.formatted_version.clone(),
                    selected_summary.is_edge,
                ));
                self.flash_report = None;
                self.post_flash_verified = false;
                self.completed_verification_note = None;
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

    pub(super) fn handle_save_cached_firmware(&mut self, path: PathBuf) -> Vec<AppCommand> {
        if self.busy.is_some() || self.step == AppStep::Updating {
            return Vec::new();
        }
        let Some(meta) = &self.cached_firmware else {
            return Vec::new();
        };
        vec![AppCommand::SaveCachedFirmware {
            target_id: meta.target_id.clone(),
            version: meta.version,
            path,
        }]
    }

    pub(super) fn handle_cached_firmware_saved(
        &mut self,
        path: PathBuf,
        result: std::result::Result<(), CoreError>,
    ) -> Vec<AppCommand> {
        match result {
            Ok(()) => {
                self.banner = Some(BannerMessage::positive(
                    "Firmware copy saved",
                    format!("Saved verified firmware binary to {}.", path.display()),
                ));
            }
            Err(err) => {
                let friendly = map_core_error(&err);
                self.banner = Some(BannerMessage::error(
                    friendly.error_title,
                    friendly.error_message,
                ));
            }
        }
        Vec::new()
    }

    pub(super) fn handle_request_cancel(&mut self) -> Vec<AppCommand> {
        if !self.busy.is_some_and(BusyOperation::is_cancellable) {
            return Vec::new();
        }
        self.banner = Some(BannerMessage::info(
            "Cancelling…",
            "Stopping the current operation cleanly.",
        ));
        vec![AppCommand::CancelActiveOperation]
    }

    pub(super) fn handle_devices_scanned(
        &mut self,
        result: std::result::Result<Vec<HidDeviceInfo>, CoreError>,
    ) -> Vec<AppCommand> {
        self.busy = None;
        self.progress = ProgressSnapshot::idle();

        match result {
            Ok(infos) => {
                let usb_infos = filter_usb_devices(&infos);
                let bt_infos = filter_bluetooth_devices(&infos);
                self.bluetooth_devices = bt_infos;
                self.pending_device_inspections.clear();

                let previous_path = self.selected_device_path.clone();
                self.devices = usb_infos.iter().map(DeviceSummary::from_hid_info).collect();

                if self.devices.is_empty() {
                    self.selected_device_path = None;
                    self.refresh_target_comparisons();
                    self.revalidate_local_firmware();
                    self.banner = Some(no_usb_devices_banner(self.bluetooth_devices.len()));
                    return Vec::new();
                }

                let selected_path = previous_path
                    .filter(|prev| self.devices.iter().any(|d| d.path == *prev))
                    .unwrap_or_else(|| self.devices[0].path.clone());
                self.selected_device_path = Some(selected_path);

                if let Some(dev) = self.selected_device().cloned() {
                    self.auto_match_target_for_device(&dev);
                }
                self.refresh_target_comparisons();
                self.revalidate_local_firmware();

                let cmds = self.queue_uninspected_usb_devices();
                if !cmds.is_empty() {
                    self.busy = Some(BusyOperation::ReadingDeviceInfo);
                    self.progress = ProgressSnapshot::indeterminate(
                        ProgressStage::ReadingController,
                        "Reading controller firmware version…",
                    );
                }
                cmds
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

    pub(super) fn handle_devices_poll_updated(
        &mut self,
        infos: Vec<HidDeviceInfo>,
    ) -> Vec<AppCommand> {
        if self.is_flashing() {
            return Vec::new();
        }

        let usb_infos = filter_usb_devices(&infos);
        let bt_infos = filter_bluetooth_devices(&infos);
        self.bluetooth_devices = bt_infos;

        let was_empty = self.devices.is_empty();
        let had_selected = self.selected_device_path.is_some();
        let selected_still_present = self
            .selected_device_path
            .as_deref()
            .is_some_and(|sel| usb_infos.iter().any(|d| d.path == sel));

        self.reconcile_devices(&usb_infos);
        self.pending_device_inspections
            .retain(|p| self.devices.iter().any(|d| d.path == *p));

        // On Completed, unplugging is expected: keep devices updated silently without leaving the page.
        if self.step == AppStep::Completed {
            if had_selected && !selected_still_present {
                self.selected_device_path = None;
                self.refresh_target_comparisons();
                self.revalidate_local_firmware();
            }
            return Vec::new();
        }

        if had_selected && !selected_still_present {
            let mut commands = Vec::new();
            if self.busy.is_some_and(BusyOperation::is_cancellable) {
                commands.push(AppCommand::CancelActiveOperation);
            }
            self.busy = None;
            self.progress = ProgressSnapshot::idle();
            self.pending_device_inspections.clear();
            self.selected_device_path = None;
            self.warning_acknowledged = false;
            self.error = None;
            self.pending_flash_attempt = None;
            self.flash_report = None;
            self.step = AppStep::DetectController;
            self.previous_step = AppStep::DetectController;
            self.refresh_target_comparisons();
            self.revalidate_local_firmware();
            self.banner = Some(BannerMessage::warning(
                "Controller disconnected",
                "Your DualSense controller was unplugged. Plug it back in with a USB cable to continue.",
            ));
            return commands;
        }

        // Recover seamlessly on Step 0 when a controller is plugged back in.
        if self.step == AppStep::DetectController
            && self.busy.is_none()
            && was_empty
            && !self.devices.is_empty()
        {
            let first = self.devices[0].clone();
            self.selected_device_path = Some(first.path.clone());
            self.warning_acknowledged = false;
            self.auto_match_target_for_device(&first);
            self.refresh_target_comparisons();
            self.revalidate_local_firmware();
            self.busy = Some(BusyOperation::ReadingDeviceInfo);
            self.progress = ProgressSnapshot::indeterminate(
                ProgressStage::ReadingController,
                "Reading controller firmware version…",
            );
            return self.queue_uninspected_usb_devices();
        }

        if self.step == AppStep::DetectController && self.devices.is_empty() {
            if !was_empty || (!self.bluetooth_devices.is_empty() && self.busy.is_none()) {
                self.selected_device_path = None;
                self.refresh_target_comparisons();
                self.revalidate_local_firmware();
                self.banner = Some(no_usb_devices_banner(self.bluetooth_devices.len()));
            }
            return Vec::new();
        }

        // Auto-inspect any newly discovered USB controllers in the background without disturbing active flows.
        if self.step != AppStep::Updating {
            return self.queue_uninspected_usb_devices();
        }

        Vec::new()
    }

    fn queue_uninspected_usb_devices(&mut self) -> Vec<AppCommand> {
        let mut commands = Vec::new();
        for dev in &self.devices {
            if dev.firmware_version.is_none()
                && self.pending_device_inspections.insert(dev.path.clone())
            {
                commands.push(AppCommand::ReadDeviceFirmwareInfo {
                    device: dev.target(),
                });
            }
        }
        commands
    }

    fn reconcile_devices(&mut self, infos: &[HidDeviceInfo]) {
        let mut next = Vec::with_capacity(infos.len());
        for info in infos {
            if let Some(existing) = self.devices.iter().find(|d| {
                d.path == info.path
                    && d.vendor_id == info.vendor_id
                    && d.product_id == info.product_id
            }) {
                next.push(existing.clone());
            } else {
                next.push(DeviceSummary::from_hid_info(info));
            }
        }
        self.devices = next;
    }

    pub(super) fn handle_device_firmware_info_loaded(
        &mut self,
        path: &str,
        result: std::result::Result<DeviceInspection, CoreError>,
    ) -> Vec<AppCommand> {
        self.pending_device_inspections.remove(path);
        let is_selected = self.selected_device_path.as_deref() == Some(path);
        let was_reading_info = matches!(self.busy, Some(BusyOperation::ReadingDeviceInfo));

        if matches!(result, Err(CoreError::Cancelled)) && !was_reading_info {
            return Vec::new();
        }
        if !self.devices.iter().any(|d| d.path == path) {
            if was_reading_info
                && self
                    .selected_device()
                    .is_none_or(|d| d.firmware_version.is_some())
            {
                self.busy = None;
                self.progress = ProgressSnapshot::idle();
            }
            return Vec::new();
        }

        if was_reading_info
            && (is_selected
                || self
                    .selected_device()
                    .is_some_and(|d| d.firmware_version.is_some()))
        {
            self.busy = None;
            self.progress = ProgressSnapshot::idle();
        }

        match result {
            Ok(inspection) => {
                let fw_version_str = inspection.firmware_info.formatted_version();
                let mut loaded_device = None;
                if let Some(dev) = self.devices.iter_mut().find(|d| d.path == path) {
                    dev.apply_inspection(&inspection);
                    loaded_device = Some(dev.clone());
                }
                if is_selected && let Some(dev) = loaded_device.as_ref() {
                    self.auto_match_target_for_device(dev);
                    self.revalidate_local_firmware();
                }
                self.refresh_target_comparisons();

                if is_selected {
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

                if loaded_device.as_ref().is_some_and(|d| d.board_supported)
                    && !self.catalog_checked
                    && !self.catalog_checking
                {
                    self.catalog_checking = true;
                    return vec![AppCommand::FetchCatalog];
                }
            }
            Err(CoreError::Cancelled) => {
                if is_selected {
                    self.banner = Some(BannerMessage::info(
                        "Cancelled",
                        "Stopped reading controller firmware information.",
                    ));
                }
            }
            Err(err) => {
                if matches!(
                    err,
                    CoreError::DevicePathNotMatched(_) | CoreError::DeviceNotFound { .. }
                ) {
                    self.devices.retain(|d| d.path != path);
                    if is_selected {
                        self.selected_device_path = None;
                    }
                    self.refresh_target_comparisons();
                    self.revalidate_local_firmware();
                }
                if is_selected || self.devices.is_empty() {
                    self.enter_error(err, AppStep::DetectController);
                }
            }
        }
        Vec::new()
    }

    pub(super) fn handle_catalog_loaded(
        &mut self,
        result: std::result::Result<FirmwareCatalog, CoreError>,
    ) -> Vec<AppCommand> {
        let was_foreground = matches!(self.busy, Some(BusyOperation::CheckingCatalog));
        if matches!(result, Err(CoreError::Cancelled)) && !was_foreground {
            self.catalog_checking = false;
            return Vec::new();
        }
        self.catalog_checking = false;
        if was_foreground {
            self.busy = None;
            self.progress = ProgressSnapshot::idle();
        } else if self.catalog_checked {
            log::debug!(
                "Ignoring stale background catalog result because catalog is already loaded"
            );
            return Vec::new();
        }

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
                if was_foreground || self.step == AppStep::SelectFirmware {
                    self.refresh_catalog_banner();
                }
            }
            Err(CoreError::Cancelled) => {
                if was_foreground {
                    self.banner = Some(BannerMessage::info(
                        "Update check cancelled",
                        "Stopped checking Sony's firmware server.",
                    ));
                }
            }
            Err(err) => {
                if was_foreground {
                    let return_to = self.step;
                    self.enter_error(err, return_to);
                } else {
                    log::debug!(
                        "Background catalog check failed: {}",
                        err.technical_detail()
                    );
                }
            }
        }
        Vec::new()
    }

    pub(super) fn handle_local_file_inspected(
        &mut self,
        path: PathBuf,
        result: std::result::Result<LocalFirmwareInfo, CoreError>,
    ) -> Vec<AppCommand> {
        if matches!(result, Err(CoreError::Cancelled))
            && !matches!(self.busy, Some(BusyOperation::InspectingLocalFile))
        {
            return Vec::new();
        }
        self.busy = None;
        self.progress = ProgressSnapshot::idle();

        match result {
            Ok(info) => {
                let summary = LocalFirmwareSummary::from_info(path, &info, self.selected_device());
                self.local_firmware_info = Some(info);
                self.local_firmware = Some(summary);
                self.warning_acknowledged = false;
                self.refresh_local_firmware_banner();
            }
            Err(CoreError::Cancelled) => {
                self.banner = Some(BannerMessage::info(
                    "Cancelled",
                    "Stopped inspecting the selected firmware file.",
                ));
            }
            Err(err) => {
                self.local_firmware_info = None;
                self.local_firmware = None;
                let return_to = self.step;
                self.enter_error(err, return_to);
            }
        }
        Vec::new()
    }

    pub(super) fn handle_update_finished(
        &mut self,
        result: std::result::Result<UpdateOutcome, CoreError>,
    ) -> Vec<AppCommand> {
        if self.step != AppStep::Updating {
            return Vec::new();
        }
        let was_flashing = matches!(self.busy, Some(BusyOperation::FlashingFirmware));
        self.busy = None;

        let attempt = self.pending_flash_attempt.take();

        match result {
            Ok(outcome) => {
                let target_version = outcome.target_version;
                let verification = outcome.verification;
                let mut resulting_version_str = format!("0x{target_version:04x}");

                match verification.status {
                    PostFlashVerificationStatus::Verified => {
                        self.post_flash_verified = true;
                        if let Some(info) = &verification.firmware_info {
                            resulting_version_str = info.formatted_version();
                            if let Some(selected_path) = &self.selected_device_path
                                && let Some(dev) =
                                    self.devices.iter_mut().find(|d| d.path == *selected_path)
                            {
                                dev.apply_verified_firmware_info(info);
                            }
                        } else if let Some(selected_path) = &self.selected_device_path
                            && let Some(dev) =
                                self.devices.iter_mut().find(|d| d.path == *selected_path)
                        {
                            dev.firmware_version = Some(target_version);
                            dev.formatted_firmware_version = format!("0x{target_version:04x}");
                        }
                        self.completed_verification_note = Some(format!(
                            "Hardware verification confirmed firmware version {resulting_version_str}."
                        ));
                        self.banner = Some(BannerMessage::positive(
                            "Firmware update complete",
                            format!(
                                "Your DualSense controller is now updated to version {resulting_version_str} (verified on hardware). You can unplug and use it normally."
                            ),
                        ));
                    }
                    PostFlashVerificationStatus::Unreachable => {
                        self.post_flash_verified = false;
                        if let Some(selected_path) = &self.selected_device_path
                            && let Some(dev) =
                                self.devices.iter_mut().find(|d| d.path == *selected_path)
                        {
                            dev.firmware_version = Some(target_version);
                            dev.formatted_firmware_version = format!("0x{target_version:04x}");
                        }
                        self.completed_verification_note = Some(
                            "The controller is rebooting or re-enumerating over USB, so its updated version could not be re-read immediately. Rescan after a few seconds to confirm."
                                .to_string(),
                        );
                        self.banner = Some(BannerMessage::positive(
                            "Firmware update complete",
                            format!(
                                "Your DualSense controller was updated to version 0x{target_version:04x}. It appears to be rebooting — unplug or wait a few seconds before scanning again."
                            ),
                        ));
                    }
                    PostFlashVerificationStatus::VersionMismatch => {
                        self.post_flash_verified = false;
                        if let Some(info) = &verification.firmware_info {
                            resulting_version_str = info.formatted_version();
                            if let Some(selected_path) = &self.selected_device_path
                                && let Some(dev) =
                                    self.devices.iter_mut().find(|d| d.path == *selected_path)
                            {
                                dev.apply_verified_firmware_info(info);
                            }
                        }
                        self.completed_verification_note = Some(format!(
                            "Warning: update finished, but the controller reported firmware version {resulting_version_str} instead of expected 0x{target_version:04x}. Unplug and rescan to verify."
                        ));
                        self.banner = Some(BannerMessage::warning(
                            "Firmware update finished with version mismatch",
                            format!(
                                "The update completed, but the controller reported version {resulting_version_str} instead of 0x{target_version:04x}. Unplug the USB cable, reconnect, and scan again to verify."
                            ),
                        ));
                    }
                }

                if let Some(pending) = attempt.as_ref()
                    && pending.source_kind == FirmwareSourceKind::OfficialCatalog
                    && self.cached_firmware.is_none()
                    && let Some(target_id) = &pending.selected_target_id
                {
                    self.cached_firmware = Some(CachedFirmwareMeta {
                        target_id: target_id.clone(),
                        version: target_version,
                    });
                }

                self.refresh_target_comparisons();
                self.completed_version = Some(target_version);
                self.flash_report = attempt.as_ref().map(|pending| {
                    FlashReport::build(
                        pending,
                        &self.system_info,
                        Some(resulting_version_str),
                        None,
                        None,
                    )
                });
                self.warning_acknowledged = false;
                self.step = AppStep::Completed;
                self.progress = ProgressSnapshot::completed();
            }
            Err(CoreError::Cancelled) => {
                self.flash_report = None;
                self.post_flash_verified = false;
                self.completed_verification_note = None;
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
                self.post_flash_verified = false;
                self.completed_verification_note = None;
                self.enter_error(err, AppStep::ConfirmUpdate);
                let stage = if was_flashing {
                    "Flashing (USB HID)"
                } else {
                    "Download & Verification"
                };
                self.flash_report = attempt.as_ref().map(|pending| {
                    FlashReport::build(
                        pending,
                        &self.system_info,
                        None,
                        Some(stage),
                        self.error.as_ref(),
                    )
                });
            }
        }
        Vec::new()
    }
}

fn no_usb_devices_banner(bluetooth_count: usize) -> BannerMessage {
    if bluetooth_count > 0 {
        let detail = if bluetooth_count == 1 {
            "A DualSense controller was detected over Bluetooth. Connect it to your PC with a USB data cable to inspect and update its firmware."
        } else {
            "DualSense controllers were detected over Bluetooth. Connect one to your PC with a USB data cable to inspect and update its firmware."
        };
        BannerMessage::warning("Controller detected over Bluetooth", detail)
    } else {
        BannerMessage::warning(
            "No DualSense controller detected",
            "Plug your controller directly into your PC using a USB data cable (updates cannot run over Bluetooth), then click Scan Again.",
        )
    }
}
