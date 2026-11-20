// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cxx_qt_build::{CxxQtBuilder, QResources, QmlModule};

#[path = "src/license_report.rs"]
mod license_report;

use license_report::{
    CrateMetadata, CrateNotice, FallbackLicenseTemplates, extract_spdx_copyright_holders,
    format_application_license, format_license_bundle, is_excluded_fixture_dir,
    is_license_filename, parse_cargo_lock_packages, parse_cargo_tree_packages,
    parse_crate_package_metadata, synthesize_fallback_license_texts, validate_release_notices,
};

const EMBEDDED_DESKTOP_ICONS: &[&str] = &[
    "packaging/linux/icons/16x16/icon.png",
    "packaging/linux/icons/22x22/icon.png",
    "packaging/linux/icons/32x32/icon.png",
    "packaging/linux/icons/48x48/icon.png",
    "packaging/linux/icons/64x64/icon.png",
    "packaging/linux/icons/128x128/icon.png",
    "packaging/linux/icons/256x256/icon.png",
    "packaging/linux/icons/512x512/icon.png",
    "packaging/linux/icons/scalable/icon.svg",
];

fn main() {
    configure_qmake6();
    generate_bundled_licenses();
    println!("cargo::rerun-if-changed=src/app_icon.h");
    println!("cargo::rerun-if-changed=src/native_app.h");

    let builder = CxxQtBuilder::new_qml_module(
        QmlModule::new("io.github.azorimor.linux_dualsense_fw_updater").qml_files([
            "qml/Main.qml",
            "qml/StepIndicator.qml",
            "qml/DetectControllerStep.qml",
            "qml/SelectFirmwareStep.qml",
            "qml/ConfirmUpdateStep.qml",
            "qml/LicensesWindow.qml",
            "qml/AboutWindow.qml",
            "qml/FlashReportWindow.qml",
        ]),
    )
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .qt_module("Widgets")
    .qrc_resources(desktop_icon_resources())
    .file("src/qt_bridge.rs")
    .file("src/app_icon.rs")
    .file("src/native_app.rs")
    .cpp_file("src/app_icon.cpp")
    .cpp_file("src/native_app.cpp");

    // SAFETY: `cc_builder` is marked unsafe because it exposes cxx-qt-build's internal
    // `cc::Build` without minor-version stability guarantees; we only append a supported
    // warning-suppression flag to silence the upstream Qt 6.11.2 + GCC 16 QChar SFINAE warning.
    unsafe {
        builder.cc_builder(|cc| {
            cc.flag_if_supported("-Wno-sfinae-incomplete");
        })
    }
    .build();
}

fn configure_qmake6() {
    println!("cargo::rerun-if-env-changed=QMAKE");
    if std::env::var_os("QMAKE").is_some() {
        return;
    }

    let qmake6 = Path::new("/usr/bin/qmake6");
    if qmake6.exists() {
        // SAFETY: build.rs executes single-threaded before invoking cxx-qt-build.
        unsafe {
            std::env::set_var("QMAKE", qmake6);
        }
    }
}

fn desktop_icon_resources() -> QResources {
    QResources::from(EMBEDDED_DESKTOP_ICONS.iter().copied())
}

fn generate_bundled_licenses() {
    println!("cargo::rerun-if-changed=Cargo.toml");
    println!("cargo::rerun-if-changed=Cargo.lock");
    println!("cargo::rerun-if-changed=LICENSES/GPL-3.0-or-later.txt");
    println!("cargo::rerun-if-changed=LICENSES/MIT.txt");
    println!("cargo::rerun-if-changed=LICENSES/Apache-2.0.txt");
    println!("cargo::rerun-if-changed=LICENSES/BSD-3-Clause.txt");
    println!("cargo::rerun-if-env-changed=PROFILE");
    println!("cargo::rerun-if-env-changed=TARGET");
    println!("cargo::rerun-if-env-changed=CARGO_HOME");

    let manifest_dir = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by Cargo"),
    );
    let out_dir =
        PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR is always set by Cargo"));

    let gpl_license = fs::read_to_string(manifest_dir.join("LICENSES/GPL-3.0-or-later.txt"))
        .unwrap_or_else(|_| {
            "GNU General Public License v3.0 or later (see LICENSES/GPL-3.0-or-later.txt)"
                .to_string()
        });
    let mit_license = fs::read_to_string(manifest_dir.join("LICENSES/MIT.txt"))
        .unwrap_or_else(|_| "MIT License (see LICENSES/MIT.txt)".to_string());
    let apache2_license = fs::read_to_string(manifest_dir.join("LICENSES/Apache-2.0.txt"))
        .unwrap_or_else(|_| {
            "Apache License, Version 2.0 (see LICENSES/Apache-2.0.txt)".to_string()
        });
    let bsd3_license = fs::read_to_string(manifest_dir.join("LICENSES/BSD-3-Clause.txt"))
        .unwrap_or_else(|_| "BSD 3-Clause License (see LICENSES/BSD-3-Clause.txt)".to_string());

    let templates = FallbackLicenseTemplates {
        mit: &mit_license,
        apache2: &apache2_license,
        bsd3: &bsd3_license,
    };

    let root_license = format_application_license(&gpl_license, &mit_license);
    let include_full_texts = std::env::var("PROFILE").as_deref() == Ok("release");
    let registry_dirs = find_cargo_registry_src_dirs();

    let resolved_packages = resolve_target_packages(&manifest_dir, &registry_dirs);
    let notices: Vec<CrateNotice> = resolved_packages
        .into_iter()
        .map(|(name, version)| {
            collect_crate_notice(
                &name,
                &version,
                &registry_dirs,
                include_full_texts,
                &templates,
            )
        })
        .collect();

    if include_full_texts && let Err(errors) = validate_release_notices(&notices) {
        panic!(
            "Release license notice validation failed:\n  - {}",
            errors.join("\n  - ")
        );
    }

    let app_version = env!("CARGO_PKG_VERSION");
    let bundle = format_license_bundle(app_version, &root_license, &notices, include_full_texts);
    let generated = format!(
        "// @generated by build.rs\n\
         #[allow(dead_code)]\n\
         pub const BUNDLED_LICENSES_TEXT: &str = {bundle:?};\n"
    );

    fs::write(out_dir.join("generated_licenses.rs"), generated)
        .expect("failed to write generated_licenses.rs");
}

fn resolve_target_packages(
    manifest_dir: &Path,
    registry_dirs: &[PathBuf],
) -> Vec<(String, String)> {
    if let Some(target_pkgs) = resolve_via_cargo_tree(manifest_dir)
        && !target_pkgs.is_empty()
    {
        return target_pkgs;
    }

    // Offline/non-Cargo fallback: read Cargo.lock and retain only external crates
    // that are actually unpacked for the active target in Cargo's registry cache.
    let lock_contents = fs::read_to_string(manifest_dir.join("Cargo.lock")).unwrap_or_default();
    let mut packages = BTreeSet::new();
    for pkg in parse_cargo_lock_packages(&lock_contents) {
        if pkg.source.is_none() {
            continue;
        }
        let folder_name = format!("{}-{}", pkg.name, pkg.version);
        if registry_dirs
            .iter()
            .any(|base| base.join(&folder_name).is_dir())
        {
            packages.insert((pkg.name, pkg.version));
        }
    }
    packages.into_iter().collect()
}

fn resolve_via_cargo_tree(manifest_dir: &Path) -> Option<Vec<(String, String)>> {
    let cargo = std::env::var_os("CARGO")?;
    let target = std::env::var("TARGET").ok()?;
    let output = Command::new(cargo)
        .arg("tree")
        .arg("--offline")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(manifest_dir.join("Cargo.toml"))
        .arg("--target")
        .arg(target)
        .arg("-e")
        .arg("normal")
        .arg("--prefix")
        .arg("none")
        .arg("--format")
        .arg("{p}")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    Some(parse_cargo_tree_packages(&stdout))
}

fn find_cargo_registry_src_dirs() -> Vec<PathBuf> {
    let cargo_home = std::env::var_os("CARGO_HOME")
        .filter(|val| !val.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|val| !val.is_empty())
                .map(|home| PathBuf::from(home).join(".cargo"))
        });

    let Some(registry_src) = cargo_home.map(|home| home.join("registry").join("src")) else {
        return Vec::new();
    };

    let Ok(entries) = fs::read_dir(&registry_src) else {
        return Vec::new();
    };

    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn collect_crate_notice(
    name: &str,
    version: &str,
    registry_dirs: &[PathBuf],
    include_full_texts: bool,
    templates: &FallbackLicenseTemplates<'_>,
) -> CrateNotice {
    let folder_name = format!("{name}-{version}");
    let crate_dir = registry_dirs
        .iter()
        .map(|base| base.join(&folder_name))
        .find(|candidate| candidate.is_dir());

    let Some(crate_dir) = crate_dir else {
        return CrateNotice {
            name: name.to_string(),
            version: version.to_string(),
            metadata: CrateMetadata::default(),
            license_texts: Vec::new(),
        };
    };

    let metadata = fs::read_to_string(crate_dir.join("Cargo.toml"))
        .map(|toml| parse_crate_package_metadata(&toml))
        .unwrap_or_default();

    let license_texts = if include_full_texts {
        let mut texts = read_crate_license_files(&crate_dir, metadata.license_file.as_deref());
        if texts.is_empty() {
            let spdx_holders = fs::read_to_string(crate_dir.join("src/lib.rs"))
                .map(|src| extract_spdx_copyright_holders(&src))
                .unwrap_or_default();
            texts = synthesize_fallback_license_texts(
                name,
                metadata.license.as_deref(),
                &spdx_holders,
                templates,
            );
        }
        texts
    } else {
        Vec::new()
    };

    CrateNotice {
        name: name.to_string(),
        version: version.to_string(),
        metadata,
        license_texts,
    }
}

fn read_crate_license_files(crate_dir: &Path, license_file: Option<&str>) -> Vec<(String, String)> {
    let mut filenames = BTreeSet::new();
    collect_license_paths_recursive(crate_dir, crate_dir, &mut filenames);

    if let Some(rel) = license_file {
        let candidate = crate_dir.join(rel);
        if candidate.is_file() {
            filenames.insert(rel.replace('\\', "/"));
        }
    }

    filenames
        .into_iter()
        .filter_map(|rel| {
            let bytes = fs::read(crate_dir.join(&rel)).ok()?;
            let text = String::from_utf8_lossy(&bytes).into_owned();
            Some((rel, text))
        })
        .collect()
}

fn collect_license_paths_recursive(root: &Path, dir: &Path, out: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };

        if path.is_dir() {
            if !is_excluded_fixture_dir(name) {
                collect_license_paths_recursive(root, &path, out);
            }
            continue;
        }

        if path.is_file()
            && is_license_filename(name)
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.insert(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}
