// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQuickStyle, QString, QUrl};
use dualsense_updater::cli::{CliAction, parse_cli_args, usage_text};

const MAIN_QML_URL: &str = "qrc:/qt/qml/io/github/azorimor/linux_dualsense_fw_updater/qml/Main.qml";

fn main() {
    let cli_options = match parse_cli_args(std::env::args_os()) {
        Ok(CliAction::Help(usage)) => {
            print!("{usage}");
            std::process::exit(0);
        }
        Ok(CliAction::Run(opts)) => opts,
        Err(err) => {
            let program = std::env::args_os()
                .next()
                .map(|arg| arg.to_string_lossy().into_owned())
                .filter(|arg| !arg.is_empty())
                .unwrap_or_else(|| "dualsense-updater".to_string());
            eprintln!("Error: {err}\n\n{}", usage_text(&program));
            std::process::exit(2);
        }
    };

    init_logging();
    dualsense_updater::qt_bridge::set_startup_no_flash(cli_options.no_flash);

    // SAFETY: Executed at single-threaded startup before creating QApplication or any threads.
    unsafe {
        dualsense_updater::native_app::configure_appimage_platform_theme();
    }

    let mut app = dualsense_updater::native_app::NativeApplication::new(cli_options.qt_args);
    configure_quick_style();
    app.set_application_name("dualsense-updater");
    app.set_application_display_name("DualSense Firmware Updater");
    app.set_application_version(env!("CARGO_PKG_VERSION"));
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

    let exit_code = app.exec();
    std::process::exit(exit_code);
}

fn init_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();
}

fn configure_quick_style() {
    let qt_qml_path = dualsense_updater::app_icon::qml_imports_path();
    let import_paths = dualsense_updater::quick_style::collect_qml_import_paths(
        std::env::var_os("QML_IMPORT_PATH").as_deref(),
        std::env::var_os("QML2_IMPORT_PATH").as_deref(),
        Some(qt_qml_path.as_str()),
    );
    if let Some(style) = dualsense_updater::quick_style::resolve_quick_style(
        std::env::var_os("QT_QUICK_CONTROLS_STYLE").as_deref(),
        &import_paths,
    ) {
        QQuickStyle::set_style(&QString::from(style));
    }
    QQuickStyle::set_fallback_style(&QString::from(
        dualsense_updater::quick_style::FALLBACK_QUICK_STYLE,
    ));
}
