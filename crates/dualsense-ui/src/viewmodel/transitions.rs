// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use dualsense_core::{
    CoreError, DownloadProgress, FirmwareCatalog, HidDeviceInfo, find_known_target,
};

use crate::flash_report::{FlashReport, PendingFlashAttempt};

use super::progress::{ProgressSnapshot, ProgressStage};
use super::state::AppState;
use super::types::{
    AppCommand, AppStep, BannerMessage, BusyOperation, DeviceInspection, DeviceSummary,
    FirmwareSourceKind, LocalFirmwareInfo, LocalFirmwareSummary, TargetOption,
};

impl AppState {
    pub(super) fn handle_scan_devices(&mut self) -> Vec<AppCommand> {
        if self.busy.is_some() || self.step == AppStep::Updating {
            return Vec::new();
        }
        self.error = None;
        self.completed_version = None;
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

    pub(super) fn handle_start_update(&mut self) -> Vec<AppCommand> {
        if self.busy.is_some() || self.step != AppStep::ConfirmUpdate || !self.warning_acknowledged
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
                self.selected_device_path = Some(selected_path);

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

    pub(super) fn handle_devices_poll_updated(
        &mut self,
        infos: Vec<HidDeviceInfo>,
    ) -> Vec<AppCommand> {
        if self.is_flashing() {
            return Vec::new();
        }

        let was_empty = self.devices.is_empty();
        let had_selected = self.selected_device_path.is_some();
        let selected_still_present = self
            .selected_device_path
            .as_deref()
            .is_some_and(|sel| infos.iter().any(|d| d.path == sel));

        self.reconcile_devices(&infos);

        // On Completed, unplugging is expected: keep devices updated silently without leaving the page.
        if self.step == AppStep::Completed {
            if had_selected && !selected_still_present {
                self.selected_device_path = None;
                self.refresh_target_comparisons();
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
            self.selected_device_path = None;
            self.warning_acknowledged = false;
            self.error = None;
            self.pending_flash_attempt = None;
            self.flash_report = None;
            self.step = AppStep::DetectController;
            self.previous_step = AppStep::DetectController;
            self.refresh_target_comparisons();
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
            self.busy = Some(BusyOperation::ReadingDeviceInfo);
            self.progress = ProgressSnapshot::indeterminate(
                ProgressStage::ReadingController,
                "Reading controller firmware version…",
            );
            return vec![AppCommand::ReadDeviceFirmwareInfo {
                device: first.target(),
            }];
        }

        if self.step == AppStep::DetectController && self.devices.is_empty() && !was_empty {
            self.selected_device_path = None;
            self.refresh_target_comparisons();
            self.banner = Some(BannerMessage::warning(
                "No DualSense controller detected",
                "Plug your controller directly into your PC using a USB data cable (updates cannot run over Bluetooth), then click Scan Again.",
            ));
        }

        Vec::new()
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
        if matches!(result, Err(CoreError::Cancelled))
            && !matches!(self.busy, Some(BusyOperation::ReadingDeviceInfo))
        {
            return Vec::new();
        }
        if !self.devices.iter().any(|d| d.path == path) {
            if matches!(self.busy, Some(BusyOperation::ReadingDeviceInfo)) {
                self.busy = None;
                self.progress = ProgressSnapshot::idle();
            }
            return Vec::new();
        }

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
                if matches!(
                    err,
                    CoreError::DevicePathNotMatched(_) | CoreError::DeviceNotFound { .. }
                ) {
                    self.devices.retain(|d| d.path != path);
                    if self.selected_device_path.as_deref() == Some(path) {
                        self.selected_device_path = None;
                    }
                    self.refresh_target_comparisons();
                }
                self.enter_error(err, AppStep::DetectController);
            }
        }
        Vec::new()
    }

    pub(super) fn handle_catalog_loaded(
        &mut self,
        result: std::result::Result<FirmwareCatalog, CoreError>,
    ) -> Vec<AppCommand> {
        if matches!(result, Err(CoreError::Cancelled))
            && !matches!(self.busy, Some(BusyOperation::CheckingCatalog))
        {
            return Vec::new();
        }
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

    pub(super) fn handle_update_finished(
        &mut self,
        result: std::result::Result<u16, CoreError>,
    ) -> Vec<AppCommand> {
        if self.step != AppStep::Updating {
            return Vec::new();
        }
        let was_flashing = matches!(self.busy, Some(BusyOperation::FlashingFirmware));
        self.busy = None;

        let attempt = self.pending_flash_attempt.take();

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
                self.flash_report = attempt.as_ref().map(|pending| {
                    FlashReport::build(
                        pending,
                        &self.system_info,
                        Some(format!("0x{version:04x}")),
                        None,
                        None,
                    )
                });
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
                self.flash_report = None;
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
