// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Pure parsing and formatting helpers for Cargo.lock and crate license reports.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockPackage {
    pub name: String,
    pub version: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrateMetadata {
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub repository: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateNotice {
    pub name: String,
    pub version: String,
    pub metadata: CrateMetadata,
    pub license_texts: Vec<(String, String)>,
}

/// Parses `[[package]]` entries from a `Cargo.lock` file.
pub fn parse_cargo_lock_packages(lock_contents: &str) -> Vec<LockPackage> {
    let mut packages = Vec::new();
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut source: Option<String> = None;
    let mut in_package = false;

    for raw_line in lock_contents.lines() {
        let line = raw_line.trim();
        if line == "[[package]]" {
            if in_package && let (Some(n), Some(v)) = (name.take(), version.take()) {
                packages.push(LockPackage {
                    name: n,
                    version: v,
                    source: source.take(),
                });
            }
            in_package = true;
            name = None;
            version = None;
            source = None;
            continue;
        }

        if !in_package {
            continue;
        }

        if let Some(val) = parse_toml_string_assignment(line, "name") {
            name = Some(val);
        } else if let Some(val) = parse_toml_string_assignment(line, "version") {
            version = Some(val);
        } else if let Some(val) = parse_toml_string_assignment(line, "source") {
            source = Some(val);
        }
    }

    if in_package && let (Some(n), Some(v)) = (name, version) {
        packages.push(LockPackage {
            name: n,
            version: v,
            source,
        });
    }

    packages
}

/// Extracts `license`, `license-file`, and `repository` from a crate `Cargo.toml` `[package]` section.
pub fn parse_crate_package_metadata(cargo_toml: &str) -> CrateMetadata {
    let mut meta = CrateMetadata::default();
    let mut in_package = false;

    for raw_line in cargo_toml.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && !line.starts_with("[[") {
            let section = line.trim_start_matches('[').trim_end_matches(']').trim();
            in_package = section == "package";
            continue;
        }
        if !in_package {
            continue;
        }

        if meta.license.is_none()
            && let Some(val) = parse_toml_string_assignment(line, "license")
        {
            meta.license = Some(val);
        } else if meta.license_file.is_none()
            && let Some(val) = parse_toml_string_assignment(line, "license-file")
        {
            meta.license_file = Some(val);
        } else if meta.repository.is_none()
            && let Some(val) = parse_toml_string_assignment(line, "repository")
        {
            meta.repository = Some(val);
        }
    }

    meta
}

/// Formats the application's primary GPL-3.0-or-later and inherited MIT license notices.
pub fn format_application_license(gpl_text: &str, mit_text: &str) -> String {
    format!(
        "Linux DualSense Firmware Updater is licensed under the GNU General Public License v3.0 or later (GPL-3.0-or-later).\n\
         Copyright (C) 2026 Azorimor <azorimor@gmail.com>\n\n\
         Core DualSense USB HID protocol and firmware update logic in dualsense-core is adapted from\n\
         nchie's original CLI project (https://github.com/nchie/dualsense-updater-rs) and remains\n\
         licensed under MIT AND GPL-3.0-or-later:\n\
         Copyright (c) 2026 nchie\n\
         Copyright (c) 2026 Azorimor\n\n\
         --- LICENSES/GPL-3.0-or-later.txt ---\n{}\n\n\
         --- LICENSES/MIT.txt ---\n{}",
        gpl_text.trim(),
        mit_text.trim(),
    )
}

/// Formats the complete bundled licenses and third-party notices report.
pub fn format_license_bundle(
    app_version: &str,
    root_license: &str,
    packages: &[CrateNotice],
    include_full_texts: bool,
) -> String {
    let mut out = String::new();
    out.push_str("DualSense Firmware Updater (v");
    out.push_str(app_version);
    out.push_str(")\n");
    out.push_str("============================================================\n\n");

    out.push_str("APPLICATION LICENSE\n");
    out.push_str("------------------------------------------------------------\n");
    out.push_str(root_license.trim());
    out.push_str("\n\n");

    out.push_str("SYSTEM LIBRARY NOTICES & SOURCE AVAILABILITY\n");
    out.push_str("------------------------------------------------------------\n");
    out.push_str(
        "- Qt 6 (Qt Core, Qt Gui, Qt Qml, Qt Quick, Qt Quick Controls 2, Qt Dialogs,\n  \
         Qt Layouts, Qt ShaderTools, Qt Svg, Qt Wayland)\n  \
         Copyright (C) The Qt Company Ltd. and contributors.\n  \
         Dynamically linked; licensed under LGPL-3.0-only / GPL-3.0-only.\n  \
         Upstream source (Qt 6.8.3 in AppImage): https://download.qt.io/official_releases/qt/6.8/6.8.3/single/qt-everywhere-src-6.8.3.tar.xz\n  \
         Repository: https://code.qt.io/cgit/qt/\n\n",
    );
    out.push_str(
        "- KDE Kirigami (KDE Frameworks 6 Kirigami)\n  \
         Copyright (C) KDE e.V. and contributors.\n  \
         Dynamically loaded QML module; licensed under LGPL-2.0-or-later / LGPL-2.1-or-later / BSD-3-Clause.\n  \
         Upstream source (Kirigami 6.10.0 in AppImage): https://download.kde.org/stable/frameworks/6.10/kirigami-6.10.0.tar.xz\n  \
         Repository: https://invent.kde.org/frameworks/kirigami\n\n",
    );
    out.push_str(
        "- hidapi (system libhidapi-hidraw / hidapi)\n  \
         Alan Ott / signal11 & libusb maintainers — licensed under BSD-3-Clause / GPL-3.0 / HIDAPI license.\n  \
         Repository: https://github.com/libusb/hidapi\n\n",
    );
    out.push_str(
        "- AppImage Relinking & Written Source Offer (GPLv3 §6 / LGPLv3 §4 / LGPLv2.1 §6)\n  \
         AppImage releases bundle dynamically linked shared libraries so you can extract and\n  \
         replace/relink any `.so` library via `--appimage-extract`. Complete license texts and\n  \
         library inventories live in `usr/share/licenses/linux-dualsense-fw-updater/` inside the\n  \
         AppImage (`THIRD_PARTY_NOTICES.txt`, `BUNDLED_LIBRARIES.txt`, `LGPL-*.txt`, and\n  \
         `system-packages/*.copyright`).\n  \
         Corresponding source code and reproducible CI build scripts are published at:\n  \
         https://github.com/Azorimor/linux-dualsense-fw-updater\n  \
         Written source requests (valid for at least 3 years): azorimor@gmail.com\n\n",
    );

    out.push_str(&format!(
        "THIRD-PARTY RUST DEPENDENCIES ({} crates)\n",
        packages.len()
    ));
    out.push_str("------------------------------------------------------------\n");
    if !include_full_texts {
        out.push_str(
            "Note: Development build shows crate metadata summary. \
             Release builds embed full LICENSE/COPYING/NOTICE file texts.\n\n",
        );
    }

    for pkg in packages {
        let license_id = pkg
            .metadata
            .license
            .as_deref()
            .unwrap_or("See license file / unspecified");
        out.push_str(&format!(
            "* {} v{} — {}\n",
            pkg.name, pkg.version, license_id
        ));
        if let Some(repo) = &pkg.metadata.repository {
            out.push_str(&format!("  Repository: {repo}\n"));
        }
        if let Some(lic_file) = &pkg.metadata.license_file {
            out.push_str(&format!("  License file: {lic_file}\n"));
        }

        if include_full_texts {
            if pkg.license_texts.is_empty() {
                out.push_str("  (No LICENSE/COPYING/NOTICE file bundled in crate archive.)\n");
            } else {
                for (filename, body) in &pkg.license_texts {
                    out.push_str(&format!("\n  --- {filename} ---\n"));
                    for line in body.trim().lines() {
                        if line.is_empty() {
                            out.push('\n');
                        } else {
                            out.push_str("  ");
                            out.push_str(line);
                            out.push('\n');
                        }
                    }
                }
            }
        }
        out.push('\n');
    }

    out
}

fn parse_toml_string_assignment(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start();
    let val = rest.strip_prefix('=')?.trim_start();
    extract_quoted_string(val)
}

fn extract_quoted_string(input: &str) -> Option<String> {
    let mut chars = input.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let mut out = String::new();
    let mut escaped = false;
    for ch in chars {
        if quote == '"' && escaped {
            match ch {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                other => out.push(other),
            }
            escaped = false;
            continue;
        }
        if quote == '"' && ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Some(out);
        }
        out.push(ch);
    }
    None
}
