// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use cxx_qt_lib::{QGuiApplication, QString};
use dualsense_updater::app_icon::{DESKTOP_FILE_ID, configure_window_icon, qml_imports_path};
use dualsense_updater::native_app::NativeApplication;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn build_and_packaging_configure_qapplication_and_portal_platformtheme() {
    let build_rs = fs::read_to_string(repo_path("build.rs")).expect("read build.rs");
    assert!(build_rs.contains(".qt_module(\"Widgets\")"));
    assert!(build_rs.contains("src/native_app.rs"));
    assert!(build_rs.contains("src/native_app.cpp"));

    let main_rs = fs::read_to_string(repo_path("src/main.rs")).expect("read src/main.rs");
    assert!(main_rs.contains("NativeApplication::new"));
    assert!(main_rs.contains("configure_appimage_platform_theme"));
    assert!(main_rs.contains("QGuiApplication::set_desktop_file_name"));
    assert!(main_rs.contains("configure_window_icon"));

    let package_script =
        fs::read_to_string(repo_path("scripts/package-appimage.sh")).expect("read package script");
    assert!(package_script.contains("platformthemes/libqxdgdesktopportal.so"));
    assert!(package_script.contains("--deploy-deps-only"));

    let syntax = Command::new("bash")
        .arg("-n")
        .arg(repo_path("scripts/package-appimage.sh"))
        .output()
        .expect("run bash -n on package-appimage.sh");
    assert!(
        syntax.status.success(),
        "package-appimage.sh syntax check failed: {}",
        String::from_utf8_lossy(&syntax.stderr)
    );
}

#[test]
fn native_application_instantiates_qapplication_offscreen() {
    let mut app = NativeApplication::new(vec![
        "dualsense-updater".to_string(),
        "-platform".to_string(),
        "offscreen".to_string(),
    ]);

    assert!(
        app.is_widgets_application(),
        "expected QCoreApplication::instance() to be a QApplication for native file dialogs"
    );

    app.set_application_name("dualsense-updater");
    app.set_application_display_name("DualSense Firmware Updater");
    app.set_application_version(env!("CARGO_PKG_VERSION"));
    QGuiApplication::set_desktop_file_name(&QString::from(DESKTOP_FILE_ID));
    configure_window_icon();
    assert!(!qml_imports_path().is_empty());
}
