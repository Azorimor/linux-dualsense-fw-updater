// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::cancel::CancellationToken;
use crate::catalog::*;
use crate::error::{CoreError, Result};
use crate::image::FirmwareImage;
use crate::protocol::{
    BoardModel, DEFAULT_PID, DUALSENSE_EDGE_PID, FIRMWARE_PID_OFFSET, FIRMWARE_TARGET_OFFSET,
    FIRMWARE_VERSION_OFFSET, FirmwareInfo, REPORT_ID_FIRMWARE_INFO,
};

const SAMPLE_INFO_JSON: &str = r#"{"ApplicationLatestVersion":"2.2.1.2","FwUpdate0004LatestVersion":"0x0630","FwUpdate000BLatestVersion":"0x0630","FwUpdate000ELatestVersion":"0x0641","FwUpdate0044LatestVersion":"0x0217"}"#;

#[test]
fn parses_sony_info_json_and_builds_urls_with_0x_prefix() {
    let catalog = FirmwareCatalog::from_json(SAMPLE_INFO_JSON).expect("valid catalog");
    assert_eq!(
        catalog.application_latest_version.as_deref(),
        Some("2.2.1.2")
    );
    assert_eq!(catalog.releases.len(), 4);

    let bdm020 = catalog.release_for_target("0004").expect("0004 release");
    assert_eq!(bdm020.version, 0x0630);
    assert_eq!(bdm020.version_raw, "0x0630");
    assert_eq!(
        bdm020.download_url,
        "https://fwupdater.dl.playstation.net/fwupdater/fwupdate0004/0x0630/FWUPDATE0004.bin"
    );
    assert!(bdm020.target_info.expect("target info").tested);

    let bdm050 = catalog.release_for_target("000b").expect("000B release");
    assert_eq!(bdm050.target_id, "000B");
    assert_eq!(bdm050.version, 0x0630);

    let bdm060 = catalog.release_for_target("000E").expect("000E release");
    assert_eq!(bdm060.version, 0x0641);

    let edge = catalog.release_for_target("0044").expect("0044 release");
    assert_eq!(edge.version, 0x0217);
    assert!(!edge.target_info.expect("edge info").tested);

    assert_eq!(
        catalog.release_for_target("9999"),
        Err(CoreError::CatalogTargetNotFound("9999".to_string()))
    );
}

#[test]
fn rejects_invalid_json_and_invalid_versions() {
    assert!(matches!(
        FirmwareCatalog::from_json("not json"),
        Err(CoreError::CatalogParse(_))
    ));
    assert!(matches!(
        FirmwareCatalog::from_json("{}"),
        Err(CoreError::CatalogParse(_))
    ));
    assert!(matches!(
        FirmwareCatalog::from_json(r#"{"FwUpdate0004LatestVersion":"not_hex"}"#),
        Err(CoreError::CatalogParse(_))
    ));
    assert!(matches!(
        build_firmware_url("0004/../bad", "0x0630"),
        Err(CoreError::CatalogParse(_))
    ));
    assert_eq!(
        build_firmware_url("000B", "0630").expect("normalizes 0x prefix"),
        "https://fwupdater.dl.playstation.net/fwupdater/fwupdate000B/0x0630/FWUPDATE000B.bin"
    );
}

struct MockHttpClient {
    json_response: Result<String>,
    download_chunks: Vec<Vec<u8>>,
    cancel_after_first_chunk: bool,
}

impl HttpClient for MockHttpClient {
    fn get_text(&self, url: &str, _cancel: &CancellationToken) -> Result<String> {
        assert_eq!(url, SONY_INFO_JSON_URL);
        self.json_response.clone()
    }

    fn download_with_progress(
        &self,
        _url: &str,
        cancel: &CancellationToken,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<Vec<u8>> {
        let total: usize = self.download_chunks.iter().map(Vec::len).sum();
        let mut out = Vec::with_capacity(total);
        for (idx, chunk) in self.download_chunks.iter().enumerate() {
            if cancel.is_cancelled() {
                return Err(CoreError::Cancelled);
            }
            out.extend_from_slice(chunk);
            on_progress(DownloadProgress {
                downloaded_bytes: out.len(),
                total_bytes: Some(total),
            });
            if idx == 0 && self.cancel_after_first_chunk {
                cancel.cancel();
            }
        }
        Ok(out)
    }
}

fn make_official_fw_bytes(len: usize, pid: u16, target_id: u16, version: u16) -> Vec<u8> {
    let mut fw_bytes = vec![0xAAu8; len];
    let sig = b"Sony Interactive Entertainment Inc.";
    fw_bytes[..sig.len()].copy_from_slice(sig);
    fw_bytes[FIRMWARE_PID_OFFSET..FIRMWARE_PID_OFFSET + 2].copy_from_slice(&pid.to_le_bytes());
    fw_bytes[FIRMWARE_TARGET_OFFSET..FIRMWARE_TARGET_OFFSET + 2]
        .copy_from_slice(&target_id.to_le_bytes());
    fw_bytes[FIRMWARE_VERSION_OFFSET..FIRMWARE_VERSION_OFFSET + 2]
        .copy_from_slice(&version.to_le_bytes());
    fw_bytes
}

#[test]
fn catalog_client_fetches_downloads_and_validates_firmware_image() {
    let total_len = FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES + 512;
    let fw_bytes = make_official_fw_bytes(total_len, DEFAULT_PID, 0x000E, 0x0641);

    let client = FirmwareCatalogClient::new(MockHttpClient {
        json_response: Ok(SAMPLE_INFO_JSON.to_string()),
        download_chunks: vec![fw_bytes[..200].to_vec(), fw_bytes[200..].to_vec()],
        cancel_after_first_chunk: false,
    });
    let cancel = CancellationToken::new();
    let catalog = client.fetch_catalog(&cancel).expect("catalog");
    let release = catalog.release_for_target("000E").expect("000E");

    let mut progress = Vec::new();
    let image = client
        .download_firmware(release, &cancel, &mut |p| progress.push(p))
        .expect("valid image");
    assert_eq!(image.version(), 0x0641);
    assert_eq!(image.len(), total_len);
    assert_eq!(
        progress,
        vec![
            DownloadProgress {
                downloaded_bytes: 200,
                total_bytes: Some(total_len)
            },
            DownloadProgress {
                downloaded_bytes: total_len,
                total_bytes: Some(total_len)
            },
        ]
    );
}

#[test]
fn catalog_client_rejects_html_truncated_and_mismatched_downloads() {
    let cancel = CancellationToken::new();

    // 1. Small HTML error page returned with HTTP 200
    let html = b"<!DOCTYPE html><html><body>Bad Gateway</body></html>".to_vec();
    let client_html = FirmwareCatalogClient::new(MockHttpClient {
        json_response: Ok(SAMPLE_INFO_JSON.to_string()),
        download_chunks: vec![html],
        cancel_after_first_chunk: false,
    });
    let catalog = client_html.fetch_catalog(&cancel).expect("catalog");
    let release_0004 = catalog.release_for_target("0004").expect("0004");
    assert!(matches!(
        client_html.download_firmware(release_0004, &cancel, &mut |_| {}),
        Err(CoreError::InvalidDownloadedFirmware(_))
    ));

    // 2. Undersized/truncated binary (< 256 KiB) even with Sony signature
    let truncated = make_official_fw_bytes(64 * 1024, DEFAULT_PID, 0x0004, 0x0630);
    let client_truncated = FirmwareCatalogClient::new(MockHttpClient {
        json_response: Ok(SAMPLE_INFO_JSON.to_string()),
        download_chunks: vec![truncated],
        cancel_after_first_chunk: false,
    });
    assert!(matches!(
        client_truncated.download_firmware(release_0004, &cancel, &mut |_| {}),
        Err(CoreError::InvalidDownloadedFirmware(_))
    ));

    // 3. Mismatched target ID (000B payload delivered for 0004 release)
    let wrong_target = make_official_fw_bytes(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DEFAULT_PID,
        0x000B,
        0x0630,
    );
    let client_wrong_target = FirmwareCatalogClient::new(MockHttpClient {
        json_response: Ok(SAMPLE_INFO_JSON.to_string()),
        download_chunks: vec![wrong_target],
        cancel_after_first_chunk: false,
    });
    assert_eq!(
        client_wrong_target.download_firmware(release_0004, &cancel, &mut |_| {}),
        Err(CoreError::FirmwareTargetMismatch {
            expected: "0004".to_string(),
            actual: "000B".to_string(),
        })
    );

    // 4. Mismatched version (0x0520 payload delivered for 0x0630 release)
    let wrong_version = make_official_fw_bytes(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DEFAULT_PID,
        0x0004,
        0x0520,
    );
    let client_wrong_version = FirmwareCatalogClient::new(MockHttpClient {
        json_response: Ok(SAMPLE_INFO_JSON.to_string()),
        download_chunks: vec![wrong_version],
        cancel_after_first_chunk: false,
    });
    assert_eq!(
        client_wrong_version.download_firmware(release_0004, &cancel, &mut |_| {}),
        Err(CoreError::FirmwareVersionMismatch {
            expected: 0x0630,
            actual: 0x0520,
        })
    );
}

#[test]
fn catalog_client_respects_cancellation_before_and_during_download() {
    let client = FirmwareCatalogClient::new(MockHttpClient {
        json_response: Ok(SAMPLE_INFO_JSON.to_string()),
        download_chunks: vec![vec![0u8; 128], vec![0u8; 128]],
        cancel_after_first_chunk: true,
    });
    let cancel = CancellationToken::new();
    let catalog = client.fetch_catalog(&cancel).expect("catalog");
    let release = catalog.release_for_target("0004").expect("0004");

    let err = client
        .download_firmware(release, &cancel, &mut |_| {})
        .unwrap_err();
    assert_eq!(err, CoreError::Cancelled);
    assert_eq!(client.fetch_catalog(&cancel), Err(CoreError::Cancelled));
}

fn make_hw_firmware_info(
    fw_type: u16,
    fw_series: u16,
    hw_info: u32,
    fw_version: u32,
    upd_version: u16,
) -> FirmwareInfo {
    let mut raw = vec![0u8; 64];
    raw[0] = REPORT_ID_FIRMWARE_INFO;
    raw[1..12].copy_from_slice(b"Oct  1 2024");
    raw[12..20].copy_from_slice(b"12:00:00");
    raw[20..22].copy_from_slice(&fw_type.to_le_bytes());
    raw[22..24].copy_from_slice(&fw_series.to_le_bytes());
    raw[24..28].copy_from_slice(&hw_info.to_le_bytes());
    raw[28..32].copy_from_slice(&fw_version.to_le_bytes());
    raw[44..46].copy_from_slice(&upd_version.to_le_bytes());
    FirmwareInfo::from_report(raw).expect("valid firmware info")
}

#[test]
fn resolve_controller_target_maps_all_supported_boards_and_edge() {
    let bdm050 = make_hw_firmware_info(0x0003, 0x000B, 0x0000_0811, 0x0001_0630, 0x0630);
    let hw050 = resolve_controller_target(&bdm050, DEFAULT_PID);
    assert_eq!(hw050.board_model, BoardModel::Bdm050);
    assert_eq!(hw050.board_name, "BDM-050");
    assert_eq!(hw050.fw_series, 0x000B);
    assert_eq!(hw050.fw_series_hex, "0x000B");
    assert_eq!(hw050.hw_info, 0x0000_0811);
    assert_eq!(hw050.supported_target_id, Some("000B"));
    assert!(hw050.is_officially_supported);
    assert_eq!(hw050.unsupported_reason, None);

    for (hw_info, expected_name) in [
        (0x0000_0400, "BDM-020"),
        (0x0000_0500, "BDM-030"),
        (0x0000_0600, "BDM-040"),
    ] {
        let info = make_hw_firmware_info(0x0002, 0x0004, hw_info, 0x0001_0630, 0x0630);
        let resolved = resolve_controller_target(&info, DEFAULT_PID);
        assert_eq!(resolved.board_name, expected_name);
        assert_eq!(resolved.supported_target_id, Some("0004"));
        assert!(resolved.is_officially_supported);
    }

    for (hw_info, expected_name) in [
        (0x0000_0900, "BDM-060R"),
        (0x0000_1100, "BDM-060M"),
        (0x0000_1300, "BDM-060X"),
    ] {
        let info = make_hw_firmware_info(0x0002, 0x000E, hw_info, 0x0001_0641, 0x0641);
        let resolved = resolve_controller_target(&info, DEFAULT_PID);
        assert_eq!(resolved.board_name, expected_name);
        assert_eq!(resolved.supported_target_id, Some("000E"));
        assert!(resolved.is_officially_supported);
    }

    // Known board with zeroed fw_series falls back to board mapping
    let bdm050_zero_series = make_hw_firmware_info(0, 0, 0x0000_0700, 0, 0x0500);
    let resolved_zero_series = resolve_controller_target(&bdm050_zero_series, DEFAULT_PID);
    assert_eq!(resolved_zero_series.supported_target_id, Some("000B"));
    assert!(resolved_zero_series.is_officially_supported);

    // DualSense Edge with matching 0x0044 or 0x0000 fw_series
    let edge = make_hw_firmware_info(0x0002, 0x0044, 0x0000_0400, 0x0001_0217, 0x0217);
    let hw_edge = resolve_controller_target(&edge, DUALSENSE_EDGE_PID);
    assert_eq!(hw_edge.board_name, "DualSense Edge");
    assert_eq!(hw_edge.supported_target_id, Some("0044"));
    assert!(hw_edge.is_officially_supported);

    let edge_zero = make_hw_firmware_info(0, 0, 0, 0, 0x0217);
    let hw_edge_zero = resolve_controller_target(&edge_zero, DUALSENSE_EDGE_PID);
    assert_eq!(hw_edge_zero.board_name, "DualSense Edge");
    assert_eq!(hw_edge_zero.supported_target_id, Some("0044"));
    assert!(hw_edge_zero.is_officially_supported);
}

#[test]
fn resolve_controller_target_handles_unsupported_unknown_and_mismatched_boards() {
    // BDM-010 is unsupported by Sony PC catalog
    let bdm010 = make_hw_firmware_info(0x0001, 0x0001, 0x0000_0300, 0x0001_0200, 0x0200);
    let hw010 = resolve_controller_target(&bdm010, DEFAULT_PID);
    assert_eq!(hw010.board_model, BoardModel::Bdm010);
    assert_eq!(hw010.board_name, "BDM-010");
    assert_eq!(hw010.supported_target_id, None);
    assert!(!hw010.is_officially_supported);
    assert!(
        hw010
            .unsupported_reason
            .as_deref()
            .unwrap()
            .contains("BDM-010")
    );

    // Zeroed hw_info + fw_series on standard PID -> Unknown / unsupported
    let zeroed = make_hw_firmware_info(0, 0, 0, 0, 0x0500);
    let hw_zeroed = resolve_controller_target(&zeroed, DEFAULT_PID);
    assert_eq!(hw_zeroed.board_model, BoardModel::Unknown);
    assert_eq!(hw_zeroed.board_name, "Unknown");
    assert_eq!(hw_zeroed.supported_target_id, None);
    assert!(!hw_zeroed.is_officially_supported);
    assert!(hw_zeroed.unsupported_reason.is_some());

    // Unknown board with unsupported non-zero fw_series
    let unknown_bad = make_hw_firmware_info(0, 0x0099, 0x0000_1F00, 0, 0x0500);
    let hw_unknown_bad = resolve_controller_target(&unknown_bad, DEFAULT_PID);
    assert_eq!(hw_unknown_bad.supported_target_id, None);
    assert!(!hw_unknown_bad.is_officially_supported);
    assert!(hw_unknown_bad.unsupported_reason.is_some());

    // Unknown board with known Sony fw_series resolves via fw_series
    let unknown_known_series = make_hw_firmware_info(0, 0x000E, 0x0000_1900, 0, 0x0641);
    let hw_unknown_known = resolve_controller_target(&unknown_known_series, DEFAULT_PID);
    assert_eq!(hw_unknown_known.board_name, "Unknown");
    assert_eq!(hw_unknown_known.supported_target_id, Some("000E"));
    assert!(hw_unknown_known.is_officially_supported);

    // Known board with non-zero unknown fw_series -> unsupported
    let bdm050_bad_series = make_hw_firmware_info(0, 0x0077, 0x0000_0811, 0, 0x0630);
    let hw_bad_series = resolve_controller_target(&bdm050_bad_series, DEFAULT_PID);
    assert_eq!(hw_bad_series.supported_target_id, None);
    assert!(!hw_bad_series.is_officially_supported);
    assert!(hw_bad_series.unsupported_reason.is_some());

    // Known board with mismatched known fw_series -> unsupported
    let bdm050_mismatch = make_hw_firmware_info(0, 0x0004, 0x0000_0811, 0, 0x0630);
    let hw_mismatch = resolve_controller_target(&bdm050_mismatch, DEFAULT_PID);
    assert_eq!(hw_mismatch.supported_target_id, None);
    assert!(!hw_mismatch.is_officially_supported);
    assert!(hw_mismatch.unsupported_reason.is_some());

    // DualSense Edge with mismatched fw_series -> unsupported
    let edge_mismatch = make_hw_firmware_info(0, 0x0004, 0x0000_0400, 0, 0x0217);
    let hw_edge_mismatch = resolve_controller_target(&edge_mismatch, DUALSENSE_EDGE_PID);
    assert_eq!(hw_edge_mismatch.supported_target_id, None);
    assert!(!hw_edge_mismatch.is_officially_supported);
    assert!(hw_edge_mismatch.unsupported_reason.is_some());

    // Non-Edge device reporting Edge fw_series 0x0044 -> unsupported
    let non_edge_0044 = make_hw_firmware_info(0, 0x0044, 0x0000_1F00, 0, 0x0217);
    let hw_non_edge_0044 = resolve_controller_target(&non_edge_0044, DEFAULT_PID);
    assert_eq!(hw_non_edge_0044.supported_target_id, None);
    assert!(!hw_non_edge_0044.is_officially_supported);
}
