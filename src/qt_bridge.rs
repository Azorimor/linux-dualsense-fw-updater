//! CXX-Qt QObject bridge connecting [`dualsense_ui::DefaultAppRuntime`] to QML.

use std::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList};
use dualsense_ui::{AppEvent, DefaultAppRuntime, FirmwareSourceKind, UiSnapshot};

use crate::snapshot_adapter::{
    AdaptedSnapshot, BANNER_ERROR, BANNER_INFO, parse_file_dialog_selection,
};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, step_index, READ, NOTIFY)]
        #[qproperty(QString, step_title, READ, NOTIFY)]
        #[qproperty(QString, step_subtitle, READ, NOTIFY)]
        #[qproperty(QStringList, device_paths, READ, NOTIFY)]
        #[qproperty(QStringList, device_labels, READ, NOTIFY)]
        #[qproperty(QStringList, device_details, READ, NOTIFY)]
        #[qproperty(i32, device_count, READ, NOTIFY)]
        #[qproperty(bool, has_devices, READ, NOTIFY)]
        #[qproperty(i32, selected_device_index, READ, NOTIFY)]
        #[qproperty(QString, selected_device_path, READ, NOTIFY)]
        #[qproperty(QString, selected_device_label, READ, NOTIFY)]
        #[qproperty(QString, selected_device_firmware, READ, NOTIFY)]
        #[qproperty(QString, selected_device_build_info, READ, NOTIFY)]
        #[qproperty(bool, selected_device_is_edge, READ, NOTIFY)]
        #[qproperty(QString, board_model, READ, NOTIFY)]
        #[qproperty(QString, fw_series_text, READ, NOTIFY)]
        #[qproperty(QString, hw_model_text, READ, NOTIFY)]
        #[qproperty(QString, color_name, READ, NOTIFY)]
        #[qproperty(QString, serial_number, READ, NOTIFY)]
        #[qproperty(QString, detected_target_id, READ, NOTIFY)]
        #[qproperty(bool, board_supported, READ, NOTIFY)]
        #[qproperty(bool, has_unsupported_board_warning, READ, NOTIFY)]
        #[qproperty(QString, unsupported_board_warning, READ, NOTIFY)]
        #[qproperty(bool, use_local_file, READ, NOTIFY)]
        #[qproperty(QStringList, target_ids, READ, NOTIFY)]
        #[qproperty(QStringList, target_labels, READ, NOTIFY)]
        #[qproperty(QStringList, target_version_labels, READ, NOTIFY)]
        #[qproperty(i32, selected_target_index, READ, NOTIFY)]
        #[qproperty(QString, selected_target_id, READ, NOTIFY)]
        #[qproperty(QString, selected_target_label, READ, NOTIFY)]
        #[qproperty(QString, selected_target_latest_version, READ, NOTIFY)]
        #[qproperty(bool, selected_target_tested, READ, NOTIFY)]
        #[qproperty(bool, selected_target_up_to_date, READ, NOTIFY)]
        #[qproperty(bool, catalog_checked, READ, NOTIFY)]
        #[qproperty(bool, has_local_firmware, READ, NOTIFY)]
        #[qproperty(QString, local_firmware_name, READ, NOTIFY)]
        #[qproperty(QString, local_firmware_version, READ, NOTIFY)]
        #[qproperty(QString, local_firmware_size, READ, NOTIFY)]
        #[qproperty(QString, local_firmware_path, READ, NOTIFY)]
        #[qproperty(bool, warning_acknowledged, READ, NOTIFY)]
        #[qproperty(QString, irreversible_warning, READ, NOTIFY)]
        #[qproperty(bool, has_update_summary, READ, NOTIFY)]
        #[qproperty(QString, summary_controller, READ, NOTIFY)]
        #[qproperty(QString, summary_current_version, READ, NOTIFY)]
        #[qproperty(QString, summary_target_version, READ, NOTIFY)]
        #[qproperty(QString, summary_source, READ, NOTIFY)]
        #[qproperty(bool, summary_is_untested, READ, NOTIFY)]
        #[qproperty(bool, summary_is_same_or_older, READ, NOTIFY)]
        #[qproperty(QString, completed_version, READ, NOTIFY)]
        #[qproperty(bool, has_banner, READ, NOTIFY)]
        #[qproperty(i32, banner_severity, READ, NOTIFY)]
        #[qproperty(QString, banner_title, READ, NOTIFY)]
        #[qproperty(QString, banner_text, READ, NOTIFY)]
        #[qproperty(QString, banner_display_text, READ, NOTIFY)]
        #[qproperty(bool, progress_visible, READ, NOTIFY)]
        #[qproperty(bool, progress_indeterminate, READ, NOTIFY)]
        #[qproperty(f64, progress_value, READ, NOTIFY)]
        #[qproperty(i32, progress_percentage, READ, NOTIFY)]
        #[qproperty(QString, status_title, READ, NOTIFY)]
        #[qproperty(QString, status_detail, READ, NOTIFY)]
        #[qproperty(bool, has_error, READ, NOTIFY)]
        #[qproperty(QString, error_title, READ, NOTIFY)]
        #[qproperty(QString, error_message, READ, NOTIFY)]
        #[qproperty(QString, recovery_hint, READ, NOTIFY)]
        #[qproperty(bool, is_busy, READ, NOTIFY)]
        #[qproperty(bool, can_scan, READ, NOTIFY)]
        #[qproperty(bool, can_continue_to_firmware, READ, NOTIFY)]
        #[qproperty(bool, can_select_target, READ, NOTIFY)]
        #[qproperty(bool, can_check_updates, READ, NOTIFY)]
        #[qproperty(bool, can_select_file, READ, NOTIFY)]
        #[qproperty(bool, can_review_update, READ, NOTIFY)]
        #[qproperty(bool, can_start_update, READ, NOTIFY)]
        #[qproperty(bool, can_cancel, READ, NOTIFY)]
        #[qproperty(bool, can_go_back, READ, NOTIFY)]
        type DualSenseBridge = super::DualSenseBridgeRust;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qinvokable]
        fn scan_controllers(self: &DualSenseBridge);

        #[qinvokable]
        fn select_device(self: &DualSenseBridge, path: &QString);

        #[qinvokable]
        fn select_device_at(self: &DualSenseBridge, index: i32);

        #[qinvokable]
        fn refresh_device_info(self: &DualSenseBridge);

        #[qinvokable]
        fn continue_to_firmware(self: &DualSenseBridge);

        #[qinvokable]
        fn set_use_local_file(self: &DualSenseBridge, use_local: bool);

        #[qinvokable]
        fn select_target(self: &DualSenseBridge, target_id: &QString);

        #[qinvokable]
        fn select_target_at(self: &DualSenseBridge, index: i32);

        #[qinvokable]
        fn check_for_updates(self: &DualSenseBridge);

        #[qinvokable]
        fn select_local_file(self: &DualSenseBridge, file_url: &QString);

        #[qinvokable]
        fn review_update(self: &DualSenseBridge);

        #[qinvokable]
        fn set_warning_acknowledged(self: &DualSenseBridge, acknowledged: bool);

        #[qinvokable]
        fn start_update(self: &DualSenseBridge);

        #[qinvokable]
        fn cancel_operation(self: &DualSenseBridge);

        #[qinvokable]
        fn go_back(self: &DualSenseBridge);

        #[qinvokable]
        fn dismiss_error(self: &DualSenseBridge);

        #[qinvokable]
        fn start_over(self: &DualSenseBridge);
    }

    impl cxx_qt::Threading for DualSenseBridge {}
    impl cxx_qt::Initialize for DualSenseBridge {}
}

/// Rust backing state for [`qobject::DualSenseBridge`].
pub struct DualSenseBridgeRust {
    runtime: Option<DefaultAppRuntime>,

    step_index: i32,
    step_title: QString,
    step_subtitle: QString,

    device_paths: QStringList,
    device_labels: QStringList,
    device_details: QStringList,
    device_count: i32,
    has_devices: bool,
    selected_device_index: i32,
    selected_device_path: QString,
    selected_device_label: QString,
    selected_device_firmware: QString,
    selected_device_build_info: QString,
    selected_device_is_edge: bool,
    board_model: QString,
    fw_series_text: QString,
    hw_model_text: QString,
    color_name: QString,
    serial_number: QString,
    detected_target_id: QString,
    board_supported: bool,
    has_unsupported_board_warning: bool,
    unsupported_board_warning: QString,

    use_local_file: bool,
    target_ids: QStringList,
    target_labels: QStringList,
    target_version_labels: QStringList,
    selected_target_index: i32,
    selected_target_id: QString,
    selected_target_label: QString,
    selected_target_latest_version: QString,
    selected_target_tested: bool,
    selected_target_up_to_date: bool,
    catalog_checked: bool,

    has_local_firmware: bool,
    local_firmware_name: QString,
    local_firmware_version: QString,
    local_firmware_size: QString,
    local_firmware_path: QString,

    warning_acknowledged: bool,
    irreversible_warning: QString,
    has_update_summary: bool,
    summary_controller: QString,
    summary_current_version: QString,
    summary_target_version: QString,
    summary_source: QString,
    summary_is_untested: bool,
    summary_is_same_or_older: bool,
    completed_version: QString,

    has_banner: bool,
    banner_severity: i32,
    banner_title: QString,
    banner_text: QString,
    banner_display_text: QString,

    progress_visible: bool,
    progress_indeterminate: bool,
    progress_value: f64,
    progress_percentage: i32,
    status_title: QString,
    status_detail: QString,

    has_error: bool,
    error_title: QString,
    error_message: QString,
    recovery_hint: QString,

    is_busy: bool,
    can_scan: bool,
    can_continue_to_firmware: bool,
    can_select_target: bool,
    can_check_updates: bool,
    can_select_file: bool,
    can_review_update: bool,
    can_start_update: bool,
    can_cancel: bool,
    can_go_back: bool,
}

impl Default for DualSenseBridgeRust {
    fn default() -> Self {
        Self {
            runtime: None,
            step_index: 0,
            step_title: QString::default(),
            step_subtitle: QString::default(),
            device_paths: QStringList::default(),
            device_labels: QStringList::default(),
            device_details: QStringList::default(),
            device_count: 0,
            has_devices: false,
            selected_device_index: -1,
            selected_device_path: QString::default(),
            selected_device_label: QString::default(),
            selected_device_firmware: QString::default(),
            selected_device_build_info: QString::default(),
            selected_device_is_edge: false,
            board_model: QString::default(),
            fw_series_text: QString::default(),
            hw_model_text: QString::default(),
            color_name: QString::default(),
            serial_number: QString::default(),
            detected_target_id: QString::default(),
            board_supported: false,
            has_unsupported_board_warning: false,
            unsupported_board_warning: QString::default(),
            use_local_file: false,
            target_ids: QStringList::default(),
            target_labels: QStringList::default(),
            target_version_labels: QStringList::default(),
            selected_target_index: -1,
            selected_target_id: QString::default(),
            selected_target_label: QString::default(),
            selected_target_latest_version: QString::default(),
            selected_target_tested: true,
            selected_target_up_to_date: false,
            catalog_checked: false,
            has_local_firmware: false,
            local_firmware_name: QString::default(),
            local_firmware_version: QString::default(),
            local_firmware_size: QString::default(),
            local_firmware_path: QString::default(),
            warning_acknowledged: false,
            irreversible_warning: QString::default(),
            has_update_summary: false,
            summary_controller: QString::default(),
            summary_current_version: QString::default(),
            summary_target_version: QString::default(),
            summary_source: QString::default(),
            summary_is_untested: false,
            summary_is_same_or_older: false,
            completed_version: QString::default(),
            has_banner: false,
            banner_severity: BANNER_INFO,
            banner_title: QString::default(),
            banner_text: QString::default(),
            banner_display_text: QString::default(),
            progress_visible: false,
            progress_indeterminate: false,
            progress_value: 0.0,
            progress_percentage: -1,
            status_title: QString::default(),
            status_detail: QString::default(),
            has_error: false,
            error_title: QString::default(),
            error_message: QString::default(),
            recovery_hint: QString::default(),
            is_busy: false,
            can_scan: false,
            can_continue_to_firmware: false,
            can_select_target: false,
            can_check_updates: false,
            can_select_file: false,
            can_review_update: false,
            can_start_update: false,
            can_cancel: false,
            can_go_back: false,
        }
    }
}

macro_rules! sync_prop {
    ($this:ident, $field:ident, $signal:ident, $new_val:expr) => {{
        let next = $new_val;
        if $this.rust().$field != next {
            $this.as_mut().rust_mut().$field = next;
            $this.as_mut().$signal();
        }
    }};
}

impl cxx_qt::Initialize for qobject::DualSenseBridge {
    fn initialize(mut self: Pin<&mut Self>) {
        let qt_thread = self.qt_thread();
        match DefaultAppRuntime::start(move |snapshot| {
            let _ = qt_thread.queue(move |qobject| {
                qobject.apply_snapshot(snapshot);
            });
        }) {
            Ok(runtime) => {
                let initial = runtime.snapshot();
                self.as_mut().rust_mut().runtime = Some(runtime);
                self.as_mut().apply_snapshot(initial);
                self.dispatch(AppEvent::ScanDevices);
            }
            Err(err) => {
                log::error!("Failed to initialize DualSense background runtime: {err}");
                sync_prop!(self, step_index, step_index_changed, 5);
                sync_prop!(
                    self,
                    step_title,
                    step_title_changed,
                    QString::from("Needs Attention")
                );
                sync_prop!(self, has_error, has_error_changed, true);
                sync_prop!(self, banner_severity, banner_severity_changed, BANNER_ERROR);
                sync_prop!(
                    self,
                    error_title,
                    error_title_changed,
                    QString::from("Could not start background services")
                );
                sync_prop!(
                    self,
                    error_message,
                    error_message_changed,
                    QString::from(
                        "The application could not initialize its background worker threads."
                    )
                );
                sync_prop!(
                    self,
                    recovery_hint,
                    recovery_hint_changed,
                    QString::from("Close the window and try launching the updater again.")
                );
            }
        }
    }
}

impl qobject::DualSenseBridge {
    fn dispatch(&self, event: AppEvent) {
        if let Some(runtime) = &self.rust().runtime {
            runtime.dispatch(event);
        }
    }

    fn apply_snapshot(mut self: Pin<&mut Self>, snapshot: UiSnapshot) {
        let adapted = AdaptedSnapshot::from_ui_snapshot(&snapshot);

        sync_prop!(self, step_index, step_index_changed, adapted.step_index);
        sync_prop!(
            self,
            step_title,
            step_title_changed,
            QString::from(&adapted.step_title)
        );
        sync_prop!(
            self,
            step_subtitle,
            step_subtitle_changed,
            QString::from(&adapted.step_subtitle)
        );

        sync_prop!(
            self,
            device_paths,
            device_paths_changed,
            to_qstring_list(&adapted.device_paths)
        );
        sync_prop!(
            self,
            device_labels,
            device_labels_changed,
            to_qstring_list(&adapted.device_labels)
        );
        sync_prop!(
            self,
            device_details,
            device_details_changed,
            to_qstring_list(&adapted.device_details)
        );
        sync_prop!(
            self,
            device_count,
            device_count_changed,
            adapted.device_count
        );
        sync_prop!(self, has_devices, has_devices_changed, adapted.has_devices);
        sync_prop!(
            self,
            selected_device_index,
            selected_device_index_changed,
            adapted.selected_device_index
        );
        sync_prop!(
            self,
            selected_device_path,
            selected_device_path_changed,
            QString::from(&adapted.selected_device_path)
        );
        sync_prop!(
            self,
            selected_device_label,
            selected_device_label_changed,
            QString::from(&adapted.selected_device_label)
        );
        sync_prop!(
            self,
            selected_device_firmware,
            selected_device_firmware_changed,
            QString::from(&adapted.selected_device_firmware)
        );
        sync_prop!(
            self,
            selected_device_build_info,
            selected_device_build_info_changed,
            QString::from(&adapted.selected_device_build_info)
        );
        sync_prop!(
            self,
            selected_device_is_edge,
            selected_device_is_edge_changed,
            adapted.selected_device_is_edge
        );
        sync_prop!(
            self,
            board_model,
            board_model_changed,
            QString::from(&adapted.board_model)
        );
        sync_prop!(
            self,
            fw_series_text,
            fw_series_text_changed,
            QString::from(&adapted.fw_series_text)
        );
        sync_prop!(
            self,
            hw_model_text,
            hw_model_text_changed,
            QString::from(&adapted.hw_model_text)
        );
        sync_prop!(
            self,
            color_name,
            color_name_changed,
            QString::from(&adapted.color_name)
        );
        sync_prop!(
            self,
            serial_number,
            serial_number_changed,
            QString::from(&adapted.serial_number)
        );
        sync_prop!(
            self,
            detected_target_id,
            detected_target_id_changed,
            QString::from(&adapted.detected_target_id)
        );
        sync_prop!(
            self,
            board_supported,
            board_supported_changed,
            adapted.board_supported
        );
        sync_prop!(
            self,
            has_unsupported_board_warning,
            has_unsupported_board_warning_changed,
            adapted.has_unsupported_board_warning
        );
        sync_prop!(
            self,
            unsupported_board_warning,
            unsupported_board_warning_changed,
            QString::from(&adapted.unsupported_board_warning)
        );

        sync_prop!(
            self,
            use_local_file,
            use_local_file_changed,
            adapted.use_local_file
        );
        sync_prop!(
            self,
            target_ids,
            target_ids_changed,
            to_qstring_list(&adapted.target_ids)
        );
        sync_prop!(
            self,
            target_labels,
            target_labels_changed,
            to_qstring_list(&adapted.target_labels)
        );
        sync_prop!(
            self,
            target_version_labels,
            target_version_labels_changed,
            to_qstring_list(&adapted.target_version_labels)
        );
        sync_prop!(
            self,
            selected_target_index,
            selected_target_index_changed,
            adapted.selected_target_index
        );
        sync_prop!(
            self,
            selected_target_id,
            selected_target_id_changed,
            QString::from(&adapted.selected_target_id)
        );
        sync_prop!(
            self,
            selected_target_label,
            selected_target_label_changed,
            QString::from(&adapted.selected_target_label)
        );
        sync_prop!(
            self,
            selected_target_latest_version,
            selected_target_latest_version_changed,
            QString::from(&adapted.selected_target_latest_version)
        );
        sync_prop!(
            self,
            selected_target_tested,
            selected_target_tested_changed,
            adapted.selected_target_tested
        );
        sync_prop!(
            self,
            selected_target_up_to_date,
            selected_target_up_to_date_changed,
            adapted.selected_target_up_to_date
        );
        sync_prop!(
            self,
            catalog_checked,
            catalog_checked_changed,
            adapted.catalog_checked
        );

        sync_prop!(
            self,
            has_local_firmware,
            has_local_firmware_changed,
            adapted.has_local_firmware
        );
        sync_prop!(
            self,
            local_firmware_name,
            local_firmware_name_changed,
            QString::from(&adapted.local_firmware_name)
        );
        sync_prop!(
            self,
            local_firmware_version,
            local_firmware_version_changed,
            QString::from(&adapted.local_firmware_version)
        );
        sync_prop!(
            self,
            local_firmware_size,
            local_firmware_size_changed,
            QString::from(&adapted.local_firmware_size)
        );
        sync_prop!(
            self,
            local_firmware_path,
            local_firmware_path_changed,
            QString::from(&adapted.local_firmware_path)
        );

        sync_prop!(
            self,
            warning_acknowledged,
            warning_acknowledged_changed,
            adapted.warning_acknowledged
        );
        sync_prop!(
            self,
            irreversible_warning,
            irreversible_warning_changed,
            QString::from(&adapted.irreversible_warning)
        );
        sync_prop!(
            self,
            has_update_summary,
            has_update_summary_changed,
            adapted.has_update_summary
        );
        sync_prop!(
            self,
            summary_controller,
            summary_controller_changed,
            QString::from(&adapted.summary_controller)
        );
        sync_prop!(
            self,
            summary_current_version,
            summary_current_version_changed,
            QString::from(&adapted.summary_current_version)
        );
        sync_prop!(
            self,
            summary_target_version,
            summary_target_version_changed,
            QString::from(&adapted.summary_target_version)
        );
        sync_prop!(
            self,
            summary_source,
            summary_source_changed,
            QString::from(&adapted.summary_source)
        );
        sync_prop!(
            self,
            summary_is_untested,
            summary_is_untested_changed,
            adapted.summary_is_untested
        );
        sync_prop!(
            self,
            summary_is_same_or_older,
            summary_is_same_or_older_changed,
            adapted.summary_is_same_or_older
        );
        sync_prop!(
            self,
            completed_version,
            completed_version_changed,
            QString::from(&adapted.completed_version)
        );

        sync_prop!(self, has_banner, has_banner_changed, adapted.has_banner);
        sync_prop!(
            self,
            banner_severity,
            banner_severity_changed,
            adapted.banner_severity
        );
        sync_prop!(
            self,
            banner_title,
            banner_title_changed,
            QString::from(&adapted.banner_title)
        );
        sync_prop!(
            self,
            banner_text,
            banner_text_changed,
            QString::from(&adapted.banner_text)
        );
        sync_prop!(
            self,
            banner_display_text,
            banner_display_text_changed,
            QString::from(&adapted.banner_display_text)
        );

        sync_prop!(
            self,
            progress_visible,
            progress_visible_changed,
            adapted.progress_visible
        );
        sync_prop!(
            self,
            progress_indeterminate,
            progress_indeterminate_changed,
            adapted.progress_indeterminate
        );
        if (self.rust().progress_value - adapted.progress_value).abs() > f64::EPSILON {
            self.as_mut().rust_mut().progress_value = adapted.progress_value;
            self.as_mut().progress_value_changed();
        }
        sync_prop!(
            self,
            progress_percentage,
            progress_percentage_changed,
            adapted.progress_percentage
        );
        sync_prop!(
            self,
            status_title,
            status_title_changed,
            QString::from(&adapted.status_title)
        );
        sync_prop!(
            self,
            status_detail,
            status_detail_changed,
            QString::from(&adapted.status_detail)
        );

        sync_prop!(self, has_error, has_error_changed, adapted.has_error);
        sync_prop!(
            self,
            error_title,
            error_title_changed,
            QString::from(&adapted.error_title)
        );
        sync_prop!(
            self,
            error_message,
            error_message_changed,
            QString::from(&adapted.error_message)
        );
        sync_prop!(
            self,
            recovery_hint,
            recovery_hint_changed,
            QString::from(&adapted.recovery_hint)
        );

        sync_prop!(self, is_busy, is_busy_changed, adapted.is_busy);
        sync_prop!(self, can_scan, can_scan_changed, adapted.can_scan);
        sync_prop!(
            self,
            can_continue_to_firmware,
            can_continue_to_firmware_changed,
            adapted.can_continue_to_firmware
        );
        sync_prop!(
            self,
            can_select_target,
            can_select_target_changed,
            adapted.can_select_target
        );
        sync_prop!(
            self,
            can_check_updates,
            can_check_updates_changed,
            adapted.can_check_updates
        );
        sync_prop!(
            self,
            can_select_file,
            can_select_file_changed,
            adapted.can_select_file
        );
        sync_prop!(
            self,
            can_review_update,
            can_review_update_changed,
            adapted.can_review_update
        );
        sync_prop!(
            self,
            can_start_update,
            can_start_update_changed,
            adapted.can_start_update
        );
        sync_prop!(self, can_cancel, can_cancel_changed, adapted.can_cancel);
        sync_prop!(self, can_go_back, can_go_back_changed, adapted.can_go_back);
    }

    pub fn scan_controllers(&self) {
        self.dispatch(AppEvent::ScanDevices);
    }

    pub fn select_device(&self, path: &QString) {
        self.dispatch(AppEvent::SelectDevice {
            path: path.to_string(),
        });
    }

    pub fn select_device_at(&self, index: i32) {
        if let Ok(idx) = usize::try_from(index)
            && let Some(path) = self.rust().device_paths.get(idx as isize)
        {
            self.dispatch(AppEvent::SelectDevice {
                path: path.to_string(),
            });
        }
    }

    pub fn refresh_device_info(&self) {
        self.dispatch(AppEvent::RefreshDeviceInfo);
    }

    pub fn continue_to_firmware(&self) {
        self.dispatch(AppEvent::ContinueToFirmwareStep);
    }

    pub fn set_use_local_file(&self, use_local: bool) {
        let kind = if use_local {
            FirmwareSourceKind::LocalFile
        } else {
            FirmwareSourceKind::OfficialCatalog
        };
        self.dispatch(AppEvent::SetFirmwareSourceKind(kind));
    }

    pub fn select_target(&self, target_id: &QString) {
        self.dispatch(AppEvent::SelectTarget {
            target_id: target_id.to_string(),
        });
    }

    pub fn select_target_at(&self, index: i32) {
        if let Ok(idx) = usize::try_from(index)
            && let Some(target_id) = self.rust().target_ids.get(idx as isize)
        {
            self.dispatch(AppEvent::SelectTarget {
                target_id: target_id.to_string(),
            });
        }
    }

    pub fn check_for_updates(&self) {
        self.dispatch(AppEvent::CheckForUpdates);
    }

    pub fn select_local_file(&self, file_url: &QString) {
        let raw = file_url.to_string();
        match parse_file_dialog_selection(&raw) {
            Some(path) => self.dispatch(AppEvent::SelectLocalFile { path }),
            None => log::warn!("Ignoring unsupported file dialog selection: {raw:?}"),
        }
    }

    pub fn review_update(&self) {
        self.dispatch(AppEvent::ContinueToConfirmStep);
    }

    pub fn set_warning_acknowledged(&self, acknowledged: bool) {
        self.dispatch(AppEvent::SetWarningAcknowledged(acknowledged));
    }

    pub fn start_update(&self) {
        self.dispatch(AppEvent::StartUpdate);
    }

    pub fn cancel_operation(&self) {
        self.dispatch(AppEvent::RequestCancel);
    }

    pub fn go_back(&self) {
        self.dispatch(AppEvent::GoBack);
    }

    pub fn dismiss_error(&self) {
        self.dispatch(AppEvent::DismissError);
    }

    pub fn start_over(&self) {
        self.dispatch(AppEvent::ResetFlow);
    }
}

fn to_qstring_list(values: &[String]) -> QStringList {
    values.iter().map(QString::from).collect()
}
