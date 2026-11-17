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

    out.push_str("SYSTEM LIBRARY NOTICES\n");
    out.push_str("------------------------------------------------------------\n");
    out.push_str(
        "- Qt 6 (Qt Core, Qt Gui, Qt Qml, Qt Quick, Qt Quick Controls 2)\n  \
         The Qt Company Ltd. — dynamically linked; available under LGPL-3.0-only / GPL-3.0-only.\n\n",
    );
    out.push_str(
        "- KDE Kirigami (KF6 Kirigami)\n  \
         KDE Community — dynamically loaded QML module; available under LGPL-2.0-or-later / LGPL-2.1-or-later.\n\n",
    );
    out.push_str(
        "- hidapi (system libhidapi-hidraw / hidapi)\n  \
         Alan Ott / signal11 & libusb maintainers — available under BSD-3-Clause / GPL-3.0 / HIDAPI license.\n\n",
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
