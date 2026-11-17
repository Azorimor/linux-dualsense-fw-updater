use std::path::PathBuf;

use dualsense_core::{
    CoreError, DownloadProgress, FirmwareCatalog, HidDeviceInfo, find_known_target,
};

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

    pub(super) fn handle_device_firmware_info_loaded(
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

    pub(super) fn handle_catalog_loaded(
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

    pub(super) fn handle_local_file_inspected(
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

    pub(super) fn handle_update_finished(
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
}
