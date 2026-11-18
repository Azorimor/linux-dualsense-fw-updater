// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! `QApplication` wrapper and AppImage platform-theme configuration so QtQuick
//! `FileDialog` uses the host desktop's native file picker (KDE/GTK/XDG portal).

use std::ffi::{OsStr, OsString};
use std::path::Path;

/// Platform theme plugin name used inside AppImage bundles for native portal dialogs.
pub const APPIMAGE_PLATFORM_THEME: &str = "xdgdesktopportal";

const DEFAULT_PROGRAM_NAME: &str = "dualsense-updater";

#[cxx::bridge(namespace = "dualsense_updater")]
mod ffi {
    unsafe extern "C++" {
        include!("dualsense-updater/src/native_app.h");

        type NativeApplication;

        fn create_native_application(args: Vec<String>) -> UniquePtr<NativeApplication>;
        fn exec(self: Pin<&mut NativeApplication>) -> i32;
        fn set_application_name(self: Pin<&mut NativeApplication>, name: &str);
        fn set_application_display_name(self: Pin<&mut NativeApplication>, name: &str);
        fn set_application_version(self: Pin<&mut NativeApplication>, version: &str);
        fn is_widgets_application(&self) -> bool;
    }
}

/// Owned `QApplication` instance backing the Qt event loop and widget-backed native dialogs.
pub struct NativeApplication {
    inner: cxx::UniquePtr<ffi::NativeApplication>,
}

impl NativeApplication {
    /// Instantiate a `QApplication` with normalized command-line arguments.
    pub fn new(args: Vec<String>) -> Self {
        Self {
            inner: ffi::create_native_application(args),
        }
    }

    /// Set the internal Qt application name (`QCoreApplication::setApplicationName`).
    pub fn set_application_name(&mut self, name: &str) {
        if let Some(pin) = self.inner.as_mut() {
            pin.set_application_name(name);
        }
    }

    /// Set the human-readable Qt application display name (`QGuiApplication::setApplicationDisplayName`).
    pub fn set_application_display_name(&mut self, name: &str) {
        if let Some(pin) = self.inner.as_mut() {
            pin.set_application_display_name(name);
        }
    }

    /// Set the Qt application version (`QCoreApplication::setApplicationVersion`).
    pub fn set_application_version(&mut self, version: &str) {
        if let Some(pin) = self.inner.as_mut() {
            pin.set_application_version(version);
        }
    }

    /// Return `true` when `QCoreApplication::instance()` is a `QApplication` instance.
    pub fn is_widgets_application(&self) -> bool {
        self.inner
            .as_ref()
            .is_some_and(ffi::NativeApplication::is_widgets_application)
    }

    /// Enter the Qt main event loop and return the application exit code.
    pub fn exec(&mut self) -> i32 {
        self.inner.as_mut().map(|pin| pin.exec()).unwrap_or(1)
    }
}

/// Normalize OS CLI arguments into UTF-8 strings suitable for `QApplication(argc, argv)`.
pub fn collect_cli_args<I>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut collected: Vec<String> = args
        .into_iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    if collected.is_empty() {
        collected.push(DEFAULT_PROGRAM_NAME.to_string());
    } else if collected[0].is_empty() {
        collected[0] = DEFAULT_PROGRAM_NAME.to_string();
    }
    collected
}

/// Determine whether `QT_QPA_PLATFORMTHEME` should be overridden to `"xdgdesktopportal"`
/// when executing inside an AppImage (`APPIMAGE` non-empty).
///
/// Non-AppImage launches always return `None` so native host themes (`kde`, `gtk3`, etc.)
/// remain active. Inside an AppImage, host themes like `kde` or `qt6ct` are not bundled,
/// so `"xdgdesktopportal"` is returned unless already configured (or `APPDIR` lacks the
/// bundled `libqxdgdesktopportal.so` plugin).
pub fn resolve_appimage_platform_theme(
    appimage_env: Option<&OsStr>,
    current_platform_theme: Option<&OsStr>,
    appdir_env: Option<&OsStr>,
) -> Option<&'static str> {
    let in_appimage = appimage_env.is_some_and(|val| !val.is_empty());
    if !in_appimage {
        return None;
    }

    if let Some(dir) = appdir_env.filter(|val| !val.is_empty()) {
        let portal_plugin =
            Path::new(dir).join("usr/plugins/platformthemes/libqxdgdesktopportal.so");
        if !portal_plugin.is_file() {
            return None;
        }
    }

    let already_portal = current_platform_theme
        .and_then(|val| val.to_str())
        .is_some_and(|val| {
            val.split(',')
                .next()
                .is_some_and(|first| first.trim() == APPIMAGE_PLATFORM_THEME)
        });
    if already_portal {
        return None;
    }

    Some(APPIMAGE_PLATFORM_THEME)
}

/// Apply `QT_QPA_PLATFORMTHEME=xdgdesktopportal` when running inside an AppImage bundle.
///
/// # Safety
/// Must be called during single-threaded startup before creating `QApplication` or
/// spawning background threads.
pub unsafe fn configure_appimage_platform_theme() {
    if let Some(theme) = resolve_appimage_platform_theme(
        std::env::var_os("APPIMAGE").as_deref(),
        std::env::var_os("QT_QPA_PLATFORMTHEME").as_deref(),
        std::env::var_os("APPDIR").as_deref(),
    ) {
        // SAFETY: Caller guarantees single-threaded startup before Qt/Tokio threads start.
        unsafe {
            std::env::set_var("QT_QPA_PLATFORMTHEME", theme);
        }
    }
}
