// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use crate::app_icon::qml_imports_path;
use crate::quick_style::{
    FALLBACK_QUICK_STYLE, PREFERRED_QUICK_STYLE, collect_qml_import_paths, has_qml_module,
    resolve_quick_style,
};

struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn create_temp_dir(label: &str) -> TempDirGuard {
    let unique = format!(
        "dualsense-updater-quick-style-{label}-{}-{}",
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
fn collect_qml_import_paths_splits_env_and_deduplicates_in_order() {
    let paths = collect_qml_import_paths(
        Some(OsStr::new("/first/qml::/second/qml")),
        Some(OsStr::new("/second/qml:/third/qml")),
        Some("/first/qml"),
    );
    assert_eq!(
        paths,
        vec![
            PathBuf::from("/first/qml"),
            PathBuf::from("/second/qml"),
            PathBuf::from("/third/qml"),
        ]
    );

    let with_qt = collect_qml_import_paths(None, None, Some("/usr/lib/qt6/qml"));
    assert_eq!(with_qt, vec![PathBuf::from("/usr/lib/qt6/qml")]);

    let empty = collect_qml_import_paths(Some(OsStr::new("")), None, Some(""));
    assert!(empty.is_empty());
}

#[test]
fn resolve_quick_style_prefers_org_kde_desktop_when_qmldir_exists() {
    let tmp = create_temp_dir("present");
    let desktop_dir = tmp.0.join("org/kde/desktop");
    fs::create_dir_all(&desktop_dir).expect("create org/kde/desktop dir");
    fs::write(desktop_dir.join("qmldir"), "module org.kde.desktop\n").expect("write mock qmldir");

    let paths = vec![PathBuf::from("/nonexistent"), tmp.0.clone()];
    assert!(has_qml_module(&paths, "org/kde/desktop"));
    assert_eq!(
        resolve_quick_style(None, &paths),
        Some(PREFERRED_QUICK_STYLE)
    );
    assert_eq!(
        resolve_quick_style(Some(OsStr::new("")), &paths),
        Some(PREFERRED_QUICK_STYLE)
    );
}

#[test]
fn resolve_quick_style_falls_back_to_fusion_when_org_kde_desktop_missing() {
    let tmp = create_temp_dir("missing");
    fs::create_dir_all(tmp.0.join("org/kde/desktop")).expect("create dir without qmldir");

    let paths = vec![tmp.0.clone()];
    assert!(!has_qml_module(&paths, "org/kde/desktop"));
    assert_eq!(
        resolve_quick_style(None, &paths),
        Some(FALLBACK_QUICK_STYLE)
    );
    assert_eq!(resolve_quick_style(None, &[]), Some(FALLBACK_QUICK_STYLE));
}

#[test]
fn resolve_quick_style_respects_non_empty_env_override() {
    let tmp = create_temp_dir("override");
    let desktop_dir = tmp.0.join("org/kde/desktop");
    fs::create_dir_all(&desktop_dir).expect("create org/kde/desktop dir");
    fs::write(desktop_dir.join("qmldir"), "module org.kde.desktop\n").expect("write mock qmldir");

    let paths = vec![tmp.0.clone()];
    assert_eq!(
        resolve_quick_style(Some(OsStr::new("Material")), &paths),
        None
    );
}

#[test]
fn qt_qml_imports_path_returns_non_empty_directory() {
    let path = qml_imports_path();
    assert!(
        !path.is_empty(),
        "expected QLibraryInfo::QmlImportsPath to be non-empty"
    );
    assert!(
        PathBuf::from(&path).is_dir(),
        "expected QLibraryInfo::QmlImportsPath to point to an existing directory: {path}"
    );
}
