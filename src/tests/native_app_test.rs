// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::PathBuf;

use crate::native_app::{
    APPIMAGE_PLATFORM_THEME, collect_cli_args, resolve_appimage_platform_theme,
};

struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn create_temp_dir(label: &str) -> TempDirGuard {
    let unique = format!(
        "dualsense-updater-native-app-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos()
    );
    let dir = std::env::temp_dir().join(unique);
    fs::create_dir_all(&dir).expect("create temporary test directory");
    TempDirGuard(dir)
}

#[test]
fn collect_cli_args_normalizes_empty_and_preserves_arguments() {
    assert_eq!(
        collect_cli_args(Vec::<OsString>::new()),
        vec!["dualsense-updater"]
    );
    assert_eq!(
        collect_cli_args([OsString::from("")]),
        vec!["dualsense-updater"]
    );
    assert_eq!(
        collect_cli_args([
            OsString::from("/usr/bin/dualsense-updater"),
            OsString::from("-platform"),
            OsString::from("offscreen"),
        ]),
        vec!["/usr/bin/dualsense-updater", "-platform", "offscreen"]
    );
}

#[test]
fn resolve_appimage_platform_theme_leaves_native_runs_unchanged() {
    assert_eq!(
        resolve_appimage_platform_theme(None, Some(OsStr::new("kde")), None),
        None
    );
    assert_eq!(
        resolve_appimage_platform_theme(Some(OsStr::new("")), Some(OsStr::new("kde")), None),
        None
    );
    assert_eq!(resolve_appimage_platform_theme(None, None, None), None);
}

#[test]
fn resolve_appimage_platform_theme_selects_portal_inside_appimage() {
    let appimage = Some(OsStr::new("/tmp/DualSense.AppImage"));

    assert_eq!(
        resolve_appimage_platform_theme(appimage, None, None),
        Some(APPIMAGE_PLATFORM_THEME)
    );
    assert_eq!(
        resolve_appimage_platform_theme(appimage, Some(OsStr::new("")), None),
        Some(APPIMAGE_PLATFORM_THEME)
    );
    assert_eq!(
        resolve_appimage_platform_theme(appimage, Some(OsStr::new("kde")), None),
        Some(APPIMAGE_PLATFORM_THEME)
    );
    assert_eq!(
        resolve_appimage_platform_theme(appimage, Some(OsStr::new("qt6ct")), None),
        Some(APPIMAGE_PLATFORM_THEME)
    );
    assert_eq!(
        resolve_appimage_platform_theme(appimage, Some(OsStr::new("gtk3")), None),
        Some(APPIMAGE_PLATFORM_THEME)
    );
}

#[test]
fn resolve_appimage_platform_theme_respects_existing_portal_or_missing_plugin() {
    let appimage = Some(OsStr::new("/tmp/DualSense.AppImage"));

    assert_eq!(
        resolve_appimage_platform_theme(appimage, Some(OsStr::new("xdgdesktopportal")), None),
        None
    );
    assert_eq!(
        resolve_appimage_platform_theme(appimage, Some(OsStr::new("xdgdesktopportal, kde")), None),
        None
    );

    let appdir_with_plugin = create_temp_dir("with-portal");
    let platformthemes_dir = appdir_with_plugin.0.join("usr/plugins/platformthemes");
    fs::create_dir_all(&platformthemes_dir).expect("create platformthemes dir");
    fs::write(platformthemes_dir.join("libqxdgdesktopportal.so"), b"stub")
        .expect("write stub portal plugin");

    assert_eq!(
        resolve_appimage_platform_theme(
            appimage,
            Some(OsStr::new("kde")),
            Some(appdir_with_plugin.0.as_os_str()),
        ),
        Some(APPIMAGE_PLATFORM_THEME)
    );

    let appdir_without_plugin = create_temp_dir("without-portal");
    fs::create_dir_all(appdir_without_plugin.0.join("usr/plugins/platformthemes"))
        .expect("create empty platformthemes dir");

    assert_eq!(
        resolve_appimage_platform_theme(
            appimage,
            Some(OsStr::new("kde")),
            Some(appdir_without_plugin.0.as_os_str()),
        ),
        None
    );
}
