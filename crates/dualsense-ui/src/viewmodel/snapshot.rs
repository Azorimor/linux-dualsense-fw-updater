// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use dualsense_core::LocalImageCompatibility;

use crate::error_map::FriendlyError;
use crate::flash_report::FlashReport;

use super::progress::ProgressSnapshot;
use super::state::AppState;
use super::types::{
    AppStep, BannerMessage, BusyOperation, CachedFirmwareMeta, DeviceSummary, FirmwareSourceKind,
    IRREVERSIBLE_UPDATE_WARNING, LocalFirmwareSummary, TargetOption, UpdateSummary,
};

/// Read-only snapshot rendered by QML/Kirigami or asserted in headless tests.
#[derive(Debug, Clone, PartialEq)]
pub struct UiSnapshot {
    pub step: AppStep,
    pub step_index: u8,
    pub step_title: String,
    pub devices: Vec<DeviceSummary>,
    pub bluetooth_only_detected: bool,
    pub bluetooth_device_count: usize,
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
    pub battery_summary: Option<String>,
    pub battery_percent: Option<u8>,
    pub battery_is_low: bool,
    pub battery_is_critically_low: bool,
    pub battery_warning: Option<String>,
    pub source_kind: FirmwareSourceKind,
    pub targets: Vec<TargetOption>,
    pub selected_target_id: String,
    pub selected_target: Option<TargetOption>,
    pub catalog_checked: bool,
    pub catalog_checking: bool,
    pub update_available: Option<bool>,
    pub is_up_to_date: bool,
    pub version_comparison_text: String,
    pub local_firmware: Option<LocalFirmwareSummary>,
    pub local_firmware_compatibility: Option<LocalImageCompatibility>,
    pub local_firmware_compatible: bool,
    pub local_firmware_warning: Option<String>,
    pub local_firmware_error: Option<String>,
    pub warning_acknowledged: bool,
    pub irreversible_warning: String,
    pub update_summary: Option<UpdateSummary>,
    pub completed_version: Option<String>,
    pub post_flash_verified: bool,
    pub completed_verification_note: Option<String>,
    pub has_cached_firmware: bool,
    pub cached_firmware_suggested_name: String,
    pub can_save_cached_firmware: bool,
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
    pub flash_report: Option<FlashReport>,
    pub no_flash_active: bool,
}

impl AppState {
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
            battery_summary,
            battery_percent,
            battery_is_low,
            battery_is_critically_low,
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
                dev.battery_summary.clone(),
                dev.battery_status.map(|b| b.percentage),
                dev.battery_status.is_some_and(|b| b.is_low()),
                dev.battery_status.is_some_and(|b| b.is_critically_low()),
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
                None,
                None,
                false,
                false,
            ),
        };
        let battery_warning = self.selected_device_battery_warning();

        let (error_title, error_message, recovery_hint) = match &self.error {
            Some(err) => (
                err.error_title.clone(),
                err.error_message.clone(),
                err.recovery_hint.clone(),
            ),
            None => (String::new(), String::new(), String::new()),
        };

        let bluetooth_device_count = self.bluetooth_devices.len();
        let bluetooth_only_detected = self.devices.is_empty() && bluetooth_device_count > 0;

        let local_firmware_compatibility = self.local_firmware.as_ref().map(|l| l.compatibility);
        let local_firmware_compatible = self
            .local_firmware
            .as_ref()
            .is_some_and(|l| l.compatibility != LocalImageCompatibility::Incompatible);
        let local_firmware_warning = self.local_firmware.as_ref().and_then(|l| l.warning.clone());
        let local_firmware_error = self.local_firmware.as_ref().and_then(|l| l.error.clone());

        let (update_available, is_up_to_date, version_comparison_text) = self
            .build_version_comparison(
                selected_device.as_ref(),
                selected_target.as_ref(),
                board_supported,
            );

        let has_cached_firmware = self.cached_firmware.is_some();
        let cached_firmware_suggested_name = self
            .cached_firmware
            .as_ref()
            .map(CachedFirmwareMeta::suggested_file_name)
            .unwrap_or_default();
        let can_save_cached_firmware =
            has_cached_firmware && !is_busy && self.step != AppStep::Updating;

        let update_summary = self.build_update_summary();
        let use_local_file = self.source_kind == FirmwareSourceKind::LocalFile;
        let can_scan = !is_busy && self.step != AppStep::Updating;
        let can_continue_to_firmware =
            !is_busy && self.step == AppStep::DetectController && has_device;
        let can_select_target = !is_busy && !use_local_file && board_supported;
        let can_check_updates = !is_busy
            && !self.catalog_checking
            && board_supported
            && matches!(self.step, AppStep::SelectFirmware | AppStep::ConfirmUpdate);
        let can_select_file = !is_busy
            && has_device
            && matches!(self.step, AppStep::SelectFirmware | AppStep::ConfirmUpdate);
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
        let flash_report = if matches!(self.step, AppStep::Completed | AppStep::Error) {
            self.flash_report.clone()
        } else {
            None
        };

        UiSnapshot {
            step: self.step,
            step_index: self.step.step_index(),
            step_title: self.step.title().to_string(),
            devices: self.devices.clone(),
            bluetooth_only_detected,
            bluetooth_device_count,
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
            battery_summary,
            battery_percent,
            battery_is_low,
            battery_is_critically_low,
            battery_warning,
            source_kind: self.source_kind,
            targets: self.targets.clone(),
            selected_target_id: self.selected_target_id.clone(),
            selected_target,
            catalog_checked: self.catalog_checked,
            catalog_checking: self.catalog_checking,
            update_available,
            is_up_to_date,
            version_comparison_text,
            local_firmware: self.local_firmware.clone(),
            local_firmware_compatibility,
            local_firmware_compatible,
            local_firmware_warning,
            local_firmware_error,
            warning_acknowledged: self.warning_acknowledged,
            irreversible_warning: IRREVERSIBLE_UPDATE_WARNING.to_string(),
            update_summary,
            completed_version: self.completed_version.map(|v| format!("0x{v:04x}")),
            post_flash_verified: self.post_flash_verified,
            completed_verification_note: self.completed_verification_note.clone(),
            has_cached_firmware,
            cached_firmware_suggested_name,
            can_save_cached_firmware,
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
            flash_report,
            no_flash_active: self.no_flash_active,
        }
    }

    fn build_version_comparison(
        &self,
        device: Option<&DeviceSummary>,
        target: Option<&TargetOption>,
        board_supported: bool,
    ) -> (Option<bool>, bool, String) {
        let Some(dev) = device else {
            return (None, false, String::new());
        };
        let Some(current) = dev.firmware_version else {
            return (None, false, String::new());
        };
        if !board_supported {
            return (
                None,
                false,
                format!("Current: 0x{current:04x} (no official catalog target)"),
            );
        }
        if self.catalog_checking && !self.catalog_checked {
            return (
                None,
                false,
                format!("Current: 0x{current:04x} — Checking for updates…"),
            );
        }
        if !self.catalog_checked {
            return (
                None,
                false,
                format!("Current: 0x{current:04x} (Check for updates to compare)"),
            );
        }
        let Some(latest) = target.and_then(|t| t.latest_version) else {
            return (
                None,
                false,
                format!("Current: 0x{current:04x} (No catalog release found)"),
            );
        };

        if current >= latest {
            let text = if current == latest {
                format!("Up to date (0x{current:04x})")
            } else {
                format!("Up to date (0x{current:04x}, newer than 0x{latest:04x})")
            };
            (Some(false), true, text)
        } else {
            (
                Some(true),
                false,
                format!("Update available: 0x{current:04x} → 0x{latest:04x}"),
            )
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
