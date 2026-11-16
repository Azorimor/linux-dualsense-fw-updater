use std::path::Path;

use crate::error::{CoreError, Result};
use crate::protocol::{FIRMWARE_VERSION_OFFSET, UPDATE_HEADER_LEN, WRITE_BLOCK_SIZE};

/// Validated DualSense firmware binary image loaded into memory once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareImage {
    bytes: Vec<u8>,
    version: u16,
}

impl FirmwareImage {
    /// Parses and validates a firmware image from raw bytes.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self> {
        let bytes = bytes.into();
        let version = Self::parse_version_from_slice(&bytes)?;
        if bytes.len() < UPDATE_HEADER_LEN {
            return Err(CoreError::FirmwareImageTooSmallForHeader);
        }
        Ok(Self { bytes, version })
    }

    /// Reads and validates a firmware image from a file on disk.
    pub fn from_file(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::from_bytes(bytes)
    }

    /// Extracts only the little-endian `u16` firmware version at offset `0x78`.
    pub fn parse_version_from_slice(bytes: &[u8]) -> Result<u16> {
        if bytes.len() < FIRMWARE_VERSION_OFFSET + 2 {
            return Err(CoreError::FirmwareImageTooSmall);
        }
        Ok(u16::from_le_bytes([
            bytes[FIRMWARE_VERSION_OFFSET],
            bytes[FIRMWARE_VERSION_OFFSET + 1],
        ]))
    }

    /// Returns the firmware version embedded at offset `0x78`.
    pub fn version(&self) -> u16 {
        self.version
    }

    /// Returns the firmware version formatted as `0xXXXX`.
    pub fn formatted_version(&self) -> String {
        format!("0x{:04x}", self.version)
    }

    /// Returns the 256-byte header slice used for `StartUpdate`.
    pub fn header(&self) -> &[u8] {
        &self.bytes[..UPDATE_HEADER_LEN]
    }

    /// Returns the complete firmware image bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the total byte length of the firmware image.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Returns `false` because valid firmware images are always at least 256 bytes.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Iterates over `0x8000`-byte (`32 KiB`) blocks for `WriteUpdateImage`.
    pub fn blocks(&self) -> std::slice::Chunks<'_, u8> {
        self.bytes.chunks(WRITE_BLOCK_SIZE)
    }

    /// Returns the number of `0x8000`-byte (`32 KiB`) blocks in the image.
    pub fn block_count(&self) -> usize {
        self.bytes.len().div_ceil(WRITE_BLOCK_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let file_path = temp_dir.join(format!(
            "dualsense_core_test_fw_{}.bin",
            std::process::id()
        ));
        let bytes = make_image(512, 0x0520);
        std::fs::write(&file_path, &bytes).expect("write temp firmware");

        let img = FirmwareImage::from_file(&file_path).expect("read temp firmware");
        assert_eq!(img.version(), 0x0520);
        assert_eq!(img.len(), 512);

        let _ = std::fs::remove_file(&file_path);

        let missing = FirmwareImage::from_file(&file_path);
        assert!(matches!(missing, Err(CoreError::Io(_))));
    }
}
