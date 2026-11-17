// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Runtime Qt Quick Controls style selection with graceful fallback when
//! `org.kde.desktop` (`qqc2-desktop-style`) is not installed.

use std::ffi::OsStr;
use std::path::PathBuf;

/// Preferred KDE desktop Quick Controls style when `org.kde.desktop` is present.
pub const PREFERRED_QUICK_STYLE: &str = "org.kde.desktop";

/// Built-in Qt Quick Controls fallback style when `org.kde.desktop` is absent.
pub const FALLBACK_QUICK_STYLE: &str = "Fusion";

const DESKTOP_STYLE_MODULE_PATH: &str = "org/kde/desktop";

/// Collect candidate QML import directories in Qt lookup order (`QML_IMPORT_PATH`,
/// `QML2_IMPORT_PATH`, then Qt's `QmlImportsPath`), deduplicated while preserving order.
pub fn collect_qml_import_paths(
    qml_import_path_env: Option<&OsStr>,
    qml2_import_path_env: Option<&OsStr>,
    qt_qml_imports_path: Option<&str>,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for env_val in [qml_import_path_env, qml2_import_path_env]
        .into_iter()
        .flatten()
    {
        for dir in std::env::split_paths(env_val) {
            if !dir.as_os_str().is_empty() && !paths.contains(&dir) {
                paths.push(dir);
            }
        }
    }

    if let Some(qt_path) = qt_qml_imports_path.filter(|path| !path.is_empty()) {
        let dir = PathBuf::from(qt_path);
        if !paths.contains(&dir) {
            paths.push(dir);
        }
    }

    paths
}

/// Return `true` if any QML import directory provides `<module_rel_path>/qmldir`.
pub fn has_qml_module(import_paths: &[PathBuf], module_rel_path: &str) -> bool {
    import_paths
        .iter()
        .any(|dir| dir.join(module_rel_path).join("qmldir").is_file())
}

/// Resolve the Quick Controls style to apply via `QQuickStyle::set_style`.
///
/// Returns `None` when `QT_QUICK_CONTROLS_STYLE` is explicitly set to a non-empty
/// value so Qt honors the environment override. Otherwise returns `org.kde.desktop`
/// if its QML module exists in `import_paths`, or `Fusion` if it does not.
pub fn resolve_quick_style(
    env_style_override: Option<&OsStr>,
    import_paths: &[PathBuf],
) -> Option<&'static str> {
    if env_style_override.is_some_and(|val| !val.is_empty()) {
        return None;
    }

    if has_qml_module(import_paths, DESKTOP_STYLE_MODULE_PATH) {
        Some(PREFERRED_QUICK_STYLE)
    } else {
        Some(FALLBACK_QUICK_STYLE)
    }
}
