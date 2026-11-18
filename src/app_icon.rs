// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Desktop entry identifier and embedded Qt application window icon setup.

#[cxx::bridge(namespace = "dualsense_updater")]
mod ffi {
    unsafe extern "C++" {
        include!("dualsense-updater/src/app_icon.h");

        fn configure_application_window_icon();
        fn qt_qml_imports_path() -> String;
    }
}

/// XDG desktop file identifier matching `packaging/linux/io.github.azorimor.linux_dualsense_fw_updater.desktop`.
pub const DESKTOP_FILE_ID: &str = "io.github.azorimor.linux_dualsense_fw_updater";

/// Qt resource URL for the 256x256 application icon used in QML views.
pub const EMBEDDED_APP_ICON_URL: &str = "qrc:/qt/qml/io/github/azorimor/linux_dualsense_fw_updater/packaging/linux/icons/256x256/icon.png";

/// Apply the desktop icon (from the active icon theme, falling back to embedded
/// multi-resolution Qt resources) to `QGuiApplication`.
pub fn configure_window_icon() {
    ffi::configure_application_window_icon();
}

/// Return Qt's configured `QmlImportsPath` (honoring `qt.conf` once `QGuiApplication` exists).
pub fn qml_imports_path() -> String {
    ffi::qt_qml_imports_path()
}
