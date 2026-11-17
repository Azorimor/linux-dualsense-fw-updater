// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT AND GPL-3.0-or-later

use crate::catalog::FirmwareRelease;
use crate::error::CoreError;
use crate::image::FirmwareImage;
use crate::protocol::{
    DEFAULT_PID, DUALSENSE_EDGE_PID, FIRMWARE_PID_OFFSET, FIRMWARE_TARGET_OFFSET,
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
