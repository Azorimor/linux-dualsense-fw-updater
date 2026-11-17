// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT

use crate::error::CoreError;
use crate::image::FirmwareImage;
use crate::protocol::{FIRMWARE_VERSION_OFFSET, UPDATE_HEADER_LEN, WRITE_BLOCK_SIZE};

fn make_image(len: usize, version: u16) -> Vec<u8> {
    let mut buf = vec![0xA5u8; len];
    if len >= FIRMWARE_VERSION_OFFSET + 2 {
        let le = version.to_le_bytes();
        buf[FIRMWARE_VERSION_OFFSET] = le[0];
        buf[FIRMWARE_VERSION_OFFSET + 1] = le[1];
    }
    buf
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
