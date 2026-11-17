// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT AND GPL-3.0-or-later

use std::path::Path;

use crate::catalog::FirmwareRelease;
use crate::error::{CoreError, Result};
use crate::protocol::{
    DEFAULT_PID, DUALSENSE_EDGE_PID, FIRMWARE_PID_OFFSET, FIRMWARE_TARGET_OFFSET,
    FIRMWARE_VERSION_OFFSET, UPDATE_HEADER_LEN, WRITE_BLOCK_SIZE,
};

/// Minimum expected size (`256 KiB`) of an official Sony DualSense firmware binary.
pub const MIN_OFFICIAL_FIRMWARE_BYTES: usize = 256 * 1024;

/// Number of leading header bytes searched for Sony's vendor signature.
pub const SONY_VENDOR_HEADER_LEN: usize = 0x40;

/// ASCII vendor signature present in official Sony DualSense firmware headers.
pub const SONY_HEADER_SIGNATURE: &[u8] = b"Sony Interactive Entertainment";

/// Validated DualSense firmware binary image loaded into memory once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareImage {
    bytes: Vec<u8>,
    version: u16,
}

impl FirmwareImage {
    /// Minimum expected size (`256 KiB`) of an official Sony DualSense firmware binary.
    pub const MIN_OFFICIAL_FIRMWARE_BYTES: usize = MIN_OFFICIAL_FIRMWARE_BYTES;

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

    /// Returns the USB product ID embedded at offset `0x62`.
    pub fn embedded_pid(&self) -> u16 {
        u16::from_le_bytes([
            self.bytes[FIRMWARE_PID_OFFSET],
            self.bytes[FIRMWARE_PID_OFFSET + 1],
        ])
    }

    /// Returns the firmware target/series ID embedded at offset `0x74`.
    pub fn embedded_target_id(&self) -> u16 {
        u16::from_le_bytes([
            self.bytes[FIRMWARE_TARGET_OFFSET],
            self.bytes[FIRMWARE_TARGET_OFFSET + 1],
        ])
    }

    /// Validates that the image meets official Sony DualSense binary size and header requirements.
    pub fn validate_sony_header(&self) -> Result<()> {
        if self.bytes.len() < Self::MIN_OFFICIAL_FIRMWARE_BYTES {
            return Err(CoreError::InvalidDownloadedFirmware(format!(
                "Firmware image is {} bytes, below the {} byte minimum for official Sony images",
                self.bytes.len(),
                Self::MIN_OFFICIAL_FIRMWARE_BYTES,
            )));
        }

        let vendor_header = &self.bytes[..SONY_VENDOR_HEADER_LEN];
        let has_signature = vendor_header
            .windows(SONY_HEADER_SIGNATURE.len())
            .any(|window| window == SONY_HEADER_SIGNATURE);
        if !has_signature {
            return Err(CoreError::InvalidDownloadedFirmware(
                "Missing 'Sony Interactive Entertainment' signature in firmware header 0x00..0x40"
                    .to_string(),
            ));
        }

        // Explicitly read embedded fields to verify header layout offsets are valid.
        let _pid = self.embedded_pid();
        let _target = self.embedded_target_id();
        let _version = self.version();
        Ok(())
    }

    /// Validates the Sony header and checks that version, target ID, and USB PID match `release`.
    pub fn validate_for_release(&self, release: &FirmwareRelease) -> Result<()> {
        self.validate_sony_header()?;

        if self.version() != release.version {
            return Err(CoreError::FirmwareVersionMismatch {
                expected: release.version,
                actual: self.version(),
            });
        }

        let expected_target = parse_target_id_hex(&release.target_id)?;
        let actual_target = self.embedded_target_id();
        if actual_target != expected_target {
            return Err(CoreError::FirmwareTargetMismatch {
                expected: format!("{expected_target:04X}"),
                actual: format!("{actual_target:04X}"),
            });
        }

        let actual_pid = self.embedded_pid();
        let pid_matches = if release.target_id.eq_ignore_ascii_case("0044") {
            actual_pid == DUALSENSE_EDGE_PID
        } else if release.target_id.eq_ignore_ascii_case("0004")
            || release.target_id.eq_ignore_ascii_case("000B")
            || release.target_id.eq_ignore_ascii_case("000E")
        {
            actual_pid == DEFAULT_PID
        } else {
            actual_pid == DEFAULT_PID || actual_pid == DUALSENSE_EDGE_PID
        };
        if !pid_matches {
            let expected_pid = if release.target_id.eq_ignore_ascii_case("0044") {
                DUALSENSE_EDGE_PID
            } else {
                DEFAULT_PID
            };
            return Err(CoreError::InvalidDownloadedFirmware(format!(
                "Embedded USB PID 0x{actual_pid:04x} does not match target {} (expected 0x{expected_pid:04x})",
                release.target_id,
            )));
        }

        Ok(())
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

fn parse_target_id_hex(target_id: &str) -> Result<u16> {
    let trimmed = target_id.trim();
    let hex = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    u16::from_str_radix(hex, 16).map_err(|_| {
        CoreError::InvalidDownloadedFirmware(format!(
            "Target ID {target_id:?} is not a valid hexadecimal firmware target"
        ))
    })
}
