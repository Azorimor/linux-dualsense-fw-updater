// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Opt-in verification tests against real Sony DualSense firmware `.bin` files.
//!
//! Run locally with:
//! `cargo test -p dualsense-core --features real-firmware-tests --test real_firmware_test`
//!
//! These tests are intentionally excluded from default `cargo test --workspace` and CI
//! because official Sony firmware binaries are proprietary and live only inside the
//! gitignored `tmp/` directory (or at `DUALSENSE_REAL_FW_PATH`).

#![cfg(feature = "real-firmware-tests")]

#[allow(dead_code)]
mod common;

use std::path::PathBuf;
use std::time::Duration;

use common::{NoopSleeper, SimDualSenseController};
use dualsense_core::{
    CancellationToken, CoreError, DEFAULT_PID, DUALSENSE_EDGE_PID, DownloadProgress,
    DualSenseUpdater, FirmwareCatalogClient, FirmwareImage, FirmwareRelease, HttpClient,
    SONY_FW_BASE_URL, SONY_INFO_JSON_URL, UPDATE_HEADER_LEN, UpdateConfig, WRITE_BLOCK_SIZE,
    build_firmware_url, find_known_target,
};

struct RealBinaryHttpMock {
    info_json: String,
    expected_url: String,
    bytes: Vec<u8>,
}

impl HttpClient for RealBinaryHttpMock {
    fn get_text(&self, url: &str, _cancel: &CancellationToken) -> dualsense_core::Result<String> {
        assert_eq!(url, SONY_INFO_JSON_URL);
        Ok(self.info_json.clone())
    }

    fn download_with_progress(
        &self,
        url: &str,
        _cancel: &CancellationToken,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> dualsense_core::Result<Vec<u8>> {
        assert_eq!(url, self.expected_url);
        let total = self.bytes.len();
        on_progress(DownloadProgress {
            downloaded_bytes: total,
            total_bytes: Some(total),
        });
        Ok(self.bytes.clone())
    }
}

fn discover_real_firmware_files() -> Vec<PathBuf> {
    if let Some(explicit) = std::env::var_os("DUALSENSE_REAL_FW_PATH") {
        let path = PathBuf::from(explicit);
        assert!(
            path.is_file(),
            "DUALSENSE_REAL_FW_PATH points to missing file: {}",
            path.display()
        );
        return vec![path];
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/dualsense-core has workspace root two levels up");
    let tmp_dir = repo_root.join("tmp");

    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("bin"))
            {
                files.push(path);
            }
        }
    }
    files.sort();

    assert!(
        !files.is_empty(),
        "No .bin firmware files found in {} (or DUALSENSE_REAL_FW_PATH). Place an official Sony FWUPDATE*.bin file in tmp/ before running with --features real-firmware-tests.",
        tmp_dir.display()
    );
    files
}

fn target_id_string(image: &FirmwareImage) -> String {
    format!("{:04X}", image.embedded_target_id())
}

fn release_for_real_image(image: &FirmwareImage) -> FirmwareRelease {
    let target_id = target_id_string(image);
    let version = image.version();
    let version_raw = format!("0x{version:04x}");
    FirmwareRelease {
        target_id: target_id.clone(),
        version_raw: version_raw.clone(),
        version,
        download_url: build_firmware_url(&target_id, &version_raw)
            .expect("valid release download url"),
        target_info: find_known_target(&target_id),
    }
}

#[test]
fn real_sony_firmware_passes_from_file_and_sony_header_checks() {
    for path in discover_real_firmware_files() {
        let image = FirmwareImage::from_file(&path)
            .unwrap_or_else(|e| panic!("from_file failed for {}: {e}", path.display()));
        image
            .validate_sony_header()
            .unwrap_or_else(|e| panic!("validate_sony_header failed for {}: {e}", path.display()));

        assert!(image.len() >= FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES);
        assert_eq!(image.header().len(), UPDATE_HEADER_LEN);
        assert_eq!(image.block_count(), image.len().div_ceil(WRITE_BLOCK_SIZE));

        let pid = image.embedded_pid();
        assert!(
            pid == DEFAULT_PID || pid == DUALSENSE_EDGE_PID,
            "unexpected USB PID 0x{pid:04x} in {}",
            path.display()
        );
    }
}

#[test]
fn real_sony_firmware_passes_release_validation_and_rejects_mismatches() {
    for path in discover_real_firmware_files() {
        let image = FirmwareImage::from_file(&path).expect("load real firmware");
        let matching_release = release_for_real_image(&image);
        image
            .validate_for_release(&matching_release)
            .unwrap_or_else(|e| {
                panic!(
                    "validate_for_release failed for {} ({}): {e}",
                    path.display(),
                    matching_release.target_id
                )
            });

        // Mismatched version must fail cleanly.
        let wrong_version = FirmwareRelease {
            version: matching_release.version.wrapping_add(1),
            ..matching_release.clone()
        };
        assert!(matches!(
            image.validate_for_release(&wrong_version),
            Err(CoreError::FirmwareVersionMismatch { .. })
        ));

        // Mismatched target ID must fail cleanly.
        let other_target = if matching_release.target_id == "0004" {
            "000B"
        } else {
            "0004"
        };
        let wrong_target = FirmwareRelease {
            target_id: other_target.to_string(),
            download_url: build_firmware_url(other_target, &matching_release.version_raw)
                .expect("valid wrong target url"),
            target_info: find_known_target(other_target),
            ..matching_release
        };
        assert!(matches!(
            image.validate_for_release(&wrong_target),
            Err(CoreError::FirmwareTargetMismatch { .. })
        ));
    }
}

#[test]
fn real_sony_firmware_passes_catalog_download_and_validation_pipeline() {
    for path in discover_real_firmware_files() {
        let raw_bytes = std::fs::read(&path).expect("read raw real firmware bytes");
        let loaded = FirmwareImage::from_bytes(raw_bytes.clone()).expect("parse real firmware");
        let target_id = target_id_string(&loaded);
        let version = loaded.version();
        let expected_url = format!(
            "{SONY_FW_BASE_URL}/fwupdate{target_id}/0x{version:04x}/FWUPDATE{target_id}.bin"
        );
        let info_json = format!(r#"{{"FwUpdate{target_id}LatestVersion":"0x{version:04x}"}}"#);

        let client = FirmwareCatalogClient::new(RealBinaryHttpMock {
            info_json,
            expected_url,
            bytes: raw_bytes.clone(),
        });
        let cancel = CancellationToken::new();
        let catalog = client.fetch_catalog(&cancel).expect("fetch catalog");
        let release = catalog
            .release_for_target(&target_id)
            .expect("release for embedded target");

        let mut progress_updates = Vec::new();
        let downloaded = client
            .download_firmware(release, &cancel, &mut |p| progress_updates.push(p))
            .unwrap_or_else(|e| panic!("download_firmware rejected {}: {e}", path.display()));

        assert_eq!(downloaded.len(), raw_bytes.len());
        assert_eq!(downloaded.version(), version);
        assert_eq!(progress_updates.len(), 1);
    }
}

#[test]
fn real_sony_firmware_completes_simulated_mock_hid_update() {
    let config = UpdateConfig {
        poll_interval: Duration::ZERO,
        ..UpdateConfig::default()
    };

    for path in discover_real_firmware_files() {
        let image = FirmwareImage::from_file(&path).expect("load real firmware");
        let sim = SimDualSenseController::new(image.version().saturating_sub(0x10));
        let sleeper = NoopSleeper::default();
        let updater = DualSenseUpdater::with_sleeper_and_config(&sim, sleeper, config.clone());
        let cancel = CancellationToken::new();

        let mut phases = Vec::new();
        updater
            .run_full_update(&image, &cancel, |p| phases.push(p.phase))
            .unwrap_or_else(|e| panic!("simulated HID update failed for {}: {e}", path.display()));

        assert!(*sim.finalized.borrow());
        assert_eq!(sim.received_header.borrow().as_slice(), image.header());
        assert_eq!(sim.received_image.borrow().as_slice(), image.as_bytes());
        assert!(!phases.is_empty());
    }
}
