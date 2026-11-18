// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT AND GPL-3.0-or-later

use crate::catalog::{ControllerHardwareInfo, FirmwareRelease};
use crate::error::CoreError;
use crate::image::{FirmwareImage, LocalImageCompatibility};
use crate::protocol::{
    BoardModel, DEFAULT_PID, DUALSENSE_EDGE_PID, FIRMWARE_PID_OFFSET, FIRMWARE_TARGET_OFFSET,
    FIRMWARE_VERSION_OFFSET, UPDATE_HEADER_LEN, WRITE_BLOCK_SIZE,
};

fn make_image(len: usize, version: u16) -> Vec<u8> {
    let mut buf = vec![0xA5u8; len];
    if len >= FIRMWARE_VERSION_OFFSET + 2 {
        let le = version.to_le_bytes();
        buf[FIRMWARE_VERSION_OFFSET] = le[0];
        buf[FIRMWARE_VERSION_OFFSET + 1] = le[1];
    }
    buf
}

fn make_official_image(len: usize, pid: u16, target_id: u16, version: u16) -> Vec<u8> {
    let mut buf = make_image(len, version);
    let sig = b"Sony Interactive Entertainment Inc.";
    buf[..sig.len()].copy_from_slice(sig);
    buf[FIRMWARE_PID_OFFSET..FIRMWARE_PID_OFFSET + 2].copy_from_slice(&pid.to_le_bytes());
    buf[FIRMWARE_TARGET_OFFSET..FIRMWARE_TARGET_OFFSET + 2]
        .copy_from_slice(&target_id.to_le_bytes());
    buf
}

fn make_release(target_id: &str, version: u16) -> FirmwareRelease {
    FirmwareRelease {
        target_id: target_id.to_string(),
        version_raw: format!("0x{version:04x}"),
        version,
        download_url: format!(
            "https://fwupdater.dl.playstation.net/fwupdater/fwupdate{target_id}/0x{version:04x}/FWUPDATE{target_id}.bin"
        ),
        target_info: None,
    }
}

#[test]
fn from_bytes_validates_minimum_sizes() {
    assert_eq!(
        FirmwareImage::from_bytes(vec![0u8; FIRMWARE_VERSION_OFFSET + 1]),
        Err(CoreError::FirmwareImageTooSmall)
    );
    assert_eq!(
        FirmwareImage::from_bytes(make_image(UPDATE_HEADER_LEN - 1, 0x0630)),
        Err(CoreError::FirmwareImageTooSmallForHeader)
    );
}

#[test]
fn from_bytes_extracts_version_header_and_blocks() {
    let raw = make_image(WRITE_BLOCK_SIZE + 100, 0x0641);
    let img = FirmwareImage::from_bytes(raw.clone()).expect("valid image");

    assert_eq!(img.version(), 0x0641);
    assert_eq!(img.formatted_version(), "0x0641");
    assert_eq!(img.len(), WRITE_BLOCK_SIZE + 100);
    assert!(!img.is_empty());
    assert_eq!(img.header().len(), UPDATE_HEADER_LEN);
    assert_eq!(img.header(), &raw[..UPDATE_HEADER_LEN]);
    assert_eq!(img.as_bytes(), raw.as_slice());
    assert_eq!(img.block_count(), 2);

    let blocks: Vec<&[u8]> = img.blocks().collect();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].len(), WRITE_BLOCK_SIZE);
    assert_eq!(blocks[1].len(), 100);
}

#[test]
fn from_file_reads_valid_file_and_reports_missing_file() {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("dualsense_core_test_fw_{}.bin", std::process::id()));
    let bytes = make_image(512, 0x0520);
    std::fs::write(&file_path, &bytes).expect("write temp firmware");

    let img = FirmwareImage::from_file(&file_path).expect("read temp firmware");
    assert_eq!(img.version(), 0x0520);
    assert_eq!(img.len(), 512);

    let _ = std::fs::remove_file(&file_path);

    let missing = FirmwareImage::from_file(&file_path);
    assert!(matches!(missing, Err(CoreError::Io(_))));
}

#[test]
fn validate_sony_header_rejects_undersized_and_non_sony_payloads() {
    let undersized = make_official_image(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES - 1,
        DEFAULT_PID,
        0x000B,
        0x0630,
    );
    let undersized_img = FirmwareImage::from_bytes(undersized).expect("parses raw header");
    assert!(matches!(
        undersized_img.validate_sony_header(),
        Err(CoreError::InvalidDownloadedFirmware(_))
    ));

    let mut html_error = b"<html><body>404 Not Found</body></html>".to_vec();
    html_error.resize(FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES, b' ');
    let html_img = FirmwareImage::from_bytes(html_error).expect("parses raw bytes");
    assert!(matches!(
        html_img.validate_sony_header(),
        Err(CoreError::InvalidDownloadedFirmware(_))
    ));

    let random_junk = make_image(FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES, 0x0630);
    let junk_img = FirmwareImage::from_bytes(random_junk).expect("parses raw bytes");
    assert!(matches!(
        junk_img.validate_sony_header(),
        Err(CoreError::InvalidDownloadedFirmware(_))
    ));
}

#[test]
fn validate_for_release_verifies_standard_and_edge_headers_and_rejects_mismatches() {
    let valid_std = make_official_image(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES + 128,
        DEFAULT_PID,
        0x000E,
        0x0641,
    );
    let std_img = FirmwareImage::from_bytes(valid_std).expect("valid raw image");
    assert_eq!(std_img.embedded_pid(), DEFAULT_PID);
    assert_eq!(std_img.embedded_target_id(), 0x000E);
    std_img
        .validate_for_release(&make_release("000E", 0x0641))
        .expect("matching standard image succeeds");

    let valid_edge = make_official_image(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DUALSENSE_EDGE_PID,
        0x0044,
        0x0217,
    );
    let edge_img = FirmwareImage::from_bytes(valid_edge).expect("valid raw edge image");
    edge_img
        .validate_for_release(&make_release("0044", 0x0217))
        .expect("matching edge image succeeds");

    // Version mismatch
    assert_eq!(
        std_img.validate_for_release(&make_release("000E", 0x0630)),
        Err(CoreError::FirmwareVersionMismatch {
            expected: 0x0630,
            actual: 0x0641,
        })
    );

    // Target mismatch
    assert_eq!(
        std_img.validate_for_release(&make_release("000B", 0x0641)),
        Err(CoreError::FirmwareTargetMismatch {
            expected: "000B".to_string(),
            actual: "000E".to_string(),
        })
    );

    // PID mismatch: standard PID inside Edge target "0044"
    let wrong_pid_edge = make_official_image(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DEFAULT_PID,
        0x0044,
        0x0217,
    );
    let wrong_pid_edge_img = FirmwareImage::from_bytes(wrong_pid_edge).expect("raw bytes");
    assert!(matches!(
        wrong_pid_edge_img.validate_for_release(&make_release("0044", 0x0217)),
        Err(CoreError::InvalidDownloadedFirmware(_))
    ));

    // PID mismatch: Edge PID inside standard target "0004"
    let wrong_pid_std = make_official_image(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DUALSENSE_EDGE_PID,
        0x0004,
        0x0630,
    );
    let wrong_pid_std_img = FirmwareImage::from_bytes(wrong_pid_std).expect("raw bytes");
    assert!(matches!(
        wrong_pid_std_img.validate_for_release(&make_release("0004", 0x0630)),
        Err(CoreError::InvalidDownloadedFirmware(_))
    ));
}

fn make_hardware(
    board_model: BoardModel,
    board_name: &str,
    fw_series: u16,
    supported_target_id: Option<&'static str>,
) -> ControllerHardwareInfo {
    ControllerHardwareInfo {
        board_model,
        board_name: board_name.to_string(),
        fw_type: 0x0003,
        fw_series,
        fw_series_hex: format!("0x{fw_series:04X}"),
        hw_info: 0x0000_0811,
        fw_version: 0x0001_0520,
        supported_target_id,
        is_officially_supported: supported_target_id.is_some(),
        unsupported_reason: None,
    }
}

#[test]
fn validate_for_controller_accepts_matching_standard_and_edge_images() {
    let bdm050 = make_hardware(BoardModel::Bdm050, "BDM-050", 0x000B, Some("000B"));
    let img_std = FirmwareImage::from_bytes(make_official_image(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DEFAULT_PID,
        0x000B,
        0x0630,
    ))
    .expect("valid image");

    let res = img_std.validate_for_controller(&bdm050, DEFAULT_PID);
    assert_eq!(res.compatibility, LocalImageCompatibility::Compatible);
    assert!(res.is_exact_match());
    assert!(res.is_flashable());
    assert!(res.sony_header_valid);
    assert_eq!(res.image_version, 0x0630);
    assert_eq!(res.embedded_pid, Some(DEFAULT_PID));
    assert_eq!(res.embedded_target_id.as_deref(), Some("000B"));
    assert_eq!(res.expected_pid, DEFAULT_PID);
    assert_eq!(res.expected_target_id.as_deref(), Some("000B"));
    assert!(res.mismatches.is_empty());
    assert!(res.warnings.is_empty());
    assert_eq!(res.summary(), None);

    let edge = make_hardware(BoardModel::Bdm020, "DualSense Edge", 0x0044, Some("0044"));
    let img_edge = FirmwareImage::from_bytes(make_official_image(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DUALSENSE_EDGE_PID,
        0x0044,
        0x0217,
    ))
    .expect("valid edge image");
    let edge_res = img_edge.validate_for_controller(&edge, DUALSENSE_EDGE_PID);
    assert_eq!(edge_res.compatibility, LocalImageCompatibility::Compatible);
    assert!(edge_res.is_exact_match());
}

#[test]
fn validate_for_controller_reports_header_pid_and_target_mismatches() {
    let bdm050 = make_hardware(BoardModel::Bdm050, "BDM-050", 0x000B, Some("000B"));

    // Invalid Sony header (undersized)
    let small_img = FirmwareImage::from_bytes(make_official_image(
        UPDATE_HEADER_LEN,
        DEFAULT_PID,
        0x000B,
        0x0630,
    ))
    .expect("raw bytes");
    let small_res = small_img.validate_for_controller(&bdm050, DEFAULT_PID);
    assert_eq!(
        small_res.compatibility,
        LocalImageCompatibility::Incompatible
    );
    assert!(!small_res.sony_header_valid);
    assert!(!small_res.is_flashable());
    assert_eq!(small_res.embedded_pid, None);
    assert_eq!(small_res.embedded_target_id, None);
    assert_eq!(small_res.mismatches.len(), 1);
    assert!(small_res.summary().is_some());

    // Target mismatch (000E image on BDM-050 000B controller)
    let wrong_target_img = FirmwareImage::from_bytes(make_official_image(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DEFAULT_PID,
        0x000E,
        0x0641,
    ))
    .expect("raw bytes");
    let target_res = wrong_target_img.validate_for_controller(&bdm050, DEFAULT_PID);
    assert_eq!(
        target_res.compatibility,
        LocalImageCompatibility::Incompatible
    );
    assert!(target_res.sony_header_valid);
    assert!(!target_res.is_flashable());
    assert!(
        target_res
            .mismatches
            .iter()
            .any(|m| m.contains("000E") && m.contains("000B"))
    );

    // Edge image on standard DualSense controller (both PID and target mismatch)
    let edge_img = FirmwareImage::from_bytes(make_official_image(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DUALSENSE_EDGE_PID,
        0x0044,
        0x0217,
    ))
    .expect("raw bytes");
    let pid_res = edge_img.validate_for_controller(&bdm050, DEFAULT_PID);
    assert_eq!(pid_res.compatibility, LocalImageCompatibility::Incompatible);
    assert_eq!(pid_res.mismatches.len(), 2);

    // Internal inconsistency: Edge PID inside 000B target on Edge controller
    let inconsistent_img = FirmwareImage::from_bytes(make_official_image(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DUALSENSE_EDGE_PID,
        0x000B,
        0x0630,
    ))
    .expect("raw bytes");
    let edge = make_hardware(BoardModel::Bdm020, "DualSense Edge", 0x0044, Some("0044"));
    let inc_res = inconsistent_img.validate_for_controller(&edge, DUALSENSE_EDGE_PID);
    assert_eq!(inc_res.compatibility, LocalImageCompatibility::Incompatible);
    assert!(
        inc_res
            .mismatches
            .iter()
            .any(|m| m.contains("inconsistent"))
    );
}

#[test]
fn validate_for_controller_warns_when_controller_has_no_official_target() {
    let bdm010 = make_hardware(BoardModel::Bdm010, "BDM-010", 0x0001, None);
    let img_std = FirmwareImage::from_bytes(make_official_image(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DEFAULT_PID,
        0x0004,
        0x0630,
    ))
    .expect("valid image");

    let res = img_std.validate_for_controller(&bdm010, DEFAULT_PID);
    assert_eq!(res.compatibility, LocalImageCompatibility::Warning);
    assert!(res.is_flashable());
    assert!(!res.is_exact_match());
    assert!(res.mismatches.is_empty());
    assert_eq!(res.warnings.len(), 1);
    assert!(res.summary().unwrap().contains("BDM-010"));

    // Still incompatible if PID mismatches an unverified controller
    let edge_img = FirmwareImage::from_bytes(make_official_image(
        FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES,
        DUALSENSE_EDGE_PID,
        0x0044,
        0x0217,
    ))
    .expect("valid edge image");
    let bad_res = edge_img.validate_for_controller(&bdm010, DEFAULT_PID);
    assert_eq!(bad_res.compatibility, LocalImageCompatibility::Incompatible);
    assert!(!bad_res.is_flashable());
}
