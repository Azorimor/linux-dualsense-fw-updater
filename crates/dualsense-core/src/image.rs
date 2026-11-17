// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT

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
