// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Pure parsing and formatting helpers for Cargo dependency graphs and crate license reports.

use std::collections::BTreeSet;

/// Explicit sentinel appended to the generated license bundle so binary compliance
/// checks can slice the exact UTF-8 payload without relying on `.rodata` NUL padding.
pub const LICENSE_BUNDLE_END_MARKER: &str = "=== END OF THIRD-PARTY LICENSES ===";

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

#[derive(Debug, Clone, Copy)]
pub struct FallbackLicenseTemplates<'a> {
    pub mit: &'a str,
    pub apache2: &'a str,
    pub bsd3: &'a str,
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

/// Parses deduplicated external `(name, version)` pairs from
/// `cargo tree --prefix none --format "{p}"` output, skipping local path/workspace crates.
pub fn parse_cargo_tree_packages(tree_output: &str) -> Vec<(String, String)> {
    let mut packages = BTreeSet::new();

    for raw_line in tree_output.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else {
            continue;
        };
        let Some(raw_ver) = parts.next() else {
            continue;
        };
        let Some(version) = raw_ver.strip_prefix('v') else {
            continue;
        };
        if version.is_empty() {
            continue;
        }

        // Workspace and path dependencies include a parenthesized filesystem path
        // (e.g. `(/path/to/crate)`), whereas registry crates have either no third token,
        // `(*)` for deduplicated nodes, or `(proc-macro)`.
        let is_local_path = parts
            .next()
            .is_some_and(|tok| tok.starts_with('(') && tok != "(*)" && tok != "(proc-macro)");
        if is_local_path {
            continue;
        }

        packages.insert((name.to_string(), version.to_string()));
    }

    packages.into_iter().collect()
}

/// Returns `true` when `filename` represents a license, copyright, or notice document.
pub fn is_license_filename(filename: &str) -> bool {
    let upper = filename.to_ascii_uppercase();
    // Skip REUSE sidecar annotations (e.g. `foo.rs.license`) on test fixtures.
    if upper.ends_with(".LICENSE") {
        return false;
    }
    upper.starts_with("LICENSE")
        || upper.starts_with("COPYING")
        || upper.starts_with("NOTICE")
        || upper.starts_with("COPYRIGHT")
        || upper.starts_with("UNLICENSE")
}

/// Returns `true` when `dir_name` is a non-shipping test, benchmark, or example fixture directory.
pub fn is_excluded_fixture_dir(dir_name: &str) -> bool {
    matches!(
        dir_name.to_ascii_lowercase().as_str(),
        "test"
            | "tests"
            | "test_inputs"
            | "test_outputs"
            | "bench"
            | "benches"
            | "example"
            | "examples"
            | "fuzz"
            | "fuzzing"
            | ".github"
            | "ci"
            | "doc"
            | "docs"
            | "target"
    )
}

/// Extracts unique `SPDX-FileCopyrightText:` holder lines from the header of a source file.
pub fn extract_spdx_copyright_holders(source_text: &str) -> Vec<String> {
    let mut holders = Vec::new();
    for line in source_text.lines().take(50) {
        if let Some((_, rest)) = line.split_once("SPDX-FileCopyrightText:") {
            let holder = rest.trim();
            if !holder.is_empty() && !holders.iter().any(|existing| existing == holder) {
                holders.push(holder.to_string());
            }
        }
    }
    holders
}

/// Returns a canonical upstream copyright holder for known crates whose crates.io tarballs
/// omit top-level license files (such as KDAB's `cxx-qt` family).
pub fn known_crate_copyright_holder(crate_name: &str) -> Option<&'static str> {
    match crate_name {
        "cxx-qt" | "cxx-qt-lib" | "cxx-qt-gen" | "cxx-qt-macro" | "cxx-qt-build"
        | "qt-build-utils" | "clang-format" => {
            Some("Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>")
        }
        _ => None,
    }
}

/// Synthesizes attributed fallback license notice entries when a crate archive omits
/// bundled `LICENSE*`/`COPYING*`/`NOTICE*` files.
pub fn synthesize_fallback_license_texts(
    crate_name: &str,
    license_expr: Option<&str>,
    spdx_holders: &[String],
    templates: &FallbackLicenseTemplates<'_>,
) -> Vec<(String, String)> {
    let Some(expr) = license_expr.map(str::trim).filter(|s| !s.is_empty()) else {
        return Vec::new();
    };

    let mut copyright_lines = Vec::new();
    for holder in spdx_holders {
        copyright_lines.push(format!("Copyright (C) {holder}"));
    }
    if copyright_lines.is_empty()
        && let Some(known) = known_crate_copyright_holder(crate_name)
    {
        copyright_lines.push(format!("Copyright (C) {known}"));
    }
    let copyright_header = if copyright_lines.is_empty() {
        String::new()
    } else {
        format!("{}\n\n", copyright_lines.join("\n"))
    };

    let mut entries = Vec::new();
    if expr.contains("MIT") {
        entries.push((
            "LICENSE-MIT (fallback notice)".to_string(),
            format!("{copyright_header}{}", templates.mit.trim()),
        ));
    }
    if expr.contains("Apache-2.0") {
        entries.push((
            "LICENSE-APACHE (fallback notice)".to_string(),
            format!("{copyright_header}{}", templates.apache2.trim()),
        ));
    }
    if expr.contains("BSD-3-Clause") {
        entries.push((
            "LICENSE-BSD-3-Clause (fallback notice)".to_string(),
            format!("{copyright_header}{}", templates.bsd3.trim()),
        ));
    }

    entries
}

/// Validates that every resolved crate in a release build has license metadata and notice text.
pub fn validate_release_notices(notices: &[CrateNotice]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if notices.is_empty() {
        errors.push("resolved crate notice list is empty".to_string());
        return Err(errors);
    }

    for pkg in notices {
        let has_license_meta = pkg
            .metadata
            .license
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
            || pkg
                .metadata
                .license_file
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty());
        if !has_license_meta {
            errors.push(format!(
                "{} v{}: missing license metadata",
                pkg.name, pkg.version
            ));
        }
        if pkg.license_texts.is_empty() {
            errors.push(format!(
                "{} v{}: missing license/copyright text",
                pkg.name, pkg.version
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
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
         Qt Layouts, Qt Widgets, Qt Svg, Qt Wayland)\n  \
         Copyright (C) The Qt Company Ltd. and contributors.\n  \
         Dynamically linked from the host system; licensed under LGPL-3.0-only / GPL-3.0-only\n  \
         (used under LGPL-3.0-only).\n  \
         Repository: https://code.qt.io/cgit/qt/\n\n",
    );
    out.push_str(
        "- KDE Kirigami (KDE Frameworks 6 Kirigami)\n  \
         Copyright (C) KDE e.V. and contributors.\n  \
         Dynamically loaded QML module from the host system; licensed under\n  \
         LGPL-2.0-or-later / LGPL-2.1-or-later / LGPL-3.0-only / BSD-3-Clause.\n  \
         Repository: https://invent.kde.org/frameworks/kirigami\n\n",
    );
    out.push_str(
        "- Vendored C hidapi (statically linked via the `hidapi` crate `linux-static-hidraw`)\n  \
         Copyright (C) Alan Ott / signal11 and libusb / hidapi maintainers.\n  \
         Licensed under BSD-3-Clause OR GPL-3.0-only OR LicenseRef-HIDAPI-Orig.\n  \
         Dynamically links host `libudev.so.1` from the operating system.\n  \
         Repository: https://github.com/libusb/hidapi\n\n",
    );
    out.push_str(
        "- Source Availability & Written Source Offer (GPLv3 §6 / LGPLv3 §4 / LGPLv2.1 §6)\n  \
         Official GitHub Releases distribute the portable `.tar.gz` binary archive dynamically\n  \
         linked against host Qt 6, KDE Frameworks Kirigami, `libudev`, and standard OS libraries.\n  \
         Complete Corresponding Source code and build scripts are published at:\n  \
         https://github.com/Azorimor/linux-dualsense-fw-updater\n  \
         Written source requests (valid for at least 3 years): azorimor@gmail.com\n  \
         (For optional local AppImage builds created from source via `scripts/package-appimage.sh`,\n  \
         bundled `.so` libraries can be inspected or relinked via `--appimage-extract`.)\n\n",
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

    out.push_str(LICENSE_BUNDLE_END_MARKER);
    out.push('\n');

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
