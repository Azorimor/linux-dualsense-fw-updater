// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQuickStyle, QString, QUrl};
extern crate dualsense_updater;

const MAIN_QML_URL: &str = "qrc:/qt/qml/dev/dualsense/updater/qml/Main.qml";

fn main() {
    init_logging();
    configure_quick_style();

    let mut app = QGuiApplication::new();
    if let Some(mut app_pin) = app.as_mut() {
        app_pin
            .as_mut()
            .set_application_name(&QString::from("dualsense-updater"));
        app_pin
            .as_mut()
            .set_application_display_name(&QString::from("DualSense Firmware Updater"));
        app_pin
            .as_mut()
            .set_application_version(&QString::from(env!("CARGO_PKG_VERSION")));
    }
    QGuiApplication::set_desktop_file_name(&QString::from(
        dualsense_updater::app_icon::DESKTOP_FILE_ID,
    ));
    dualsense_updater::app_icon::configure_window_icon();

    let mut engine = QQmlApplicationEngine::new();
    if let Some(mut engine_pin) = engine.as_mut() {
        engine_pin
            .as_mut()
            .on_object_creation_failed(|_, url| {
                log::error!("Failed to load QML entrypoint: {}", url.to_qstring());
                std::process::exit(1);
            })
            .release();
        engine_pin.load(&QUrl::from(MAIN_QML_URL));
    }

    let exit_code = app.as_mut().map(|app_pin| app_pin.exec()).unwrap_or(1);
    std::process::exit(exit_code);
}

fn init_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();
}

fn configure_quick_style() {
    if std::env::var_os("QT_QUICK_CONTROLS_STYLE").is_none() {
        QQuickStyle::set_style(&QString::from("org.kde.desktop"));
    }
    QQuickStyle::set_fallback_style(&QString::from("Fusion"));
}
