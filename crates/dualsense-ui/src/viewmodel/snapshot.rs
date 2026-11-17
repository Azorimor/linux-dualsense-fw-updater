// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::error_map::FriendlyError;

use super::progress::ProgressSnapshot;
use super::state::AppState;
use super::types::{
    AppStep, BannerMessage, BusyOperation, DeviceSummary, FirmwareSourceKind,
    IRREVERSIBLE_UPDATE_WARNING, LocalFirmwareSummary, TargetOption, UpdateSummary,
};

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
