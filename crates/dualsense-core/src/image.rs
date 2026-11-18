// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT AND GPL-3.0-or-later

use std::path::Path;

use crate::catalog::{ControllerHardwareInfo, FirmwareRelease, find_known_target};
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

/// Overall compatibility outcome when validating a local firmware `.bin` against a controller.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LocalImageCompatibility {
    /// Sony header is valid and embedded USB PID and hardware target ID match the controller.
    Compatible,
    /// Sony header and USB PID are valid, but the controller's hardware target could not be verified.
    Warning,
    /// Image failed Sony header validation or has a USB PID / hardware target mismatch.
    Incompatible,
}

/// Structured result of validating a local [`FirmwareImage`] against a connected controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalImageValidation {
    pub compatibility: LocalImageCompatibility,
    pub sony_header_valid: bool,
    pub image_version: u16,
    pub embedded_pid: Option<u16>,
    pub embedded_target_id: Option<String>,
    pub expected_pid: u16,
    pub expected_target_id: Option<String>,
    pub mismatches: Vec<String>,
    pub warnings: Vec<String>,
}

impl LocalImageValidation {
    /// Returns `true` when the image has no blocking mismatches (`Compatible` or `Warning`).
    pub fn is_flashable(&self) -> bool {
        !matches!(self.compatibility, LocalImageCompatibility::Incompatible)
    }

    /// Returns `true` when the image fully matches the controller with no mismatches or warnings.
    pub fn is_exact_match(&self) -> bool {
        matches!(self.compatibility, LocalImageCompatibility::Compatible)
    }

    /// Returns a concise summary of all mismatches and warnings, if any.
    pub fn summary(&self) -> Option<String> {
        let messages: Vec<&str> = self
            .mismatches
            .iter()
            .chain(self.warnings.iter())
            .map(String::as_str)
            .collect();
        if messages.is_empty() {
            None
        } else {
            Some(messages.join(" "))
        }
    }
}

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

    fn sony_header_issue(&self) -> Option<String> {
        if self.bytes.len() < Self::MIN_OFFICIAL_FIRMWARE_BYTES {
            return Some(format!(
                "Firmware image is {} bytes, below the {} byte minimum for official Sony images.",
                self.bytes.len(),
                Self::MIN_OFFICIAL_FIRMWARE_BYTES,
            ));
        }

        let vendor_header = &self.bytes[..SONY_VENDOR_HEADER_LEN];
        let has_signature = vendor_header
            .windows(SONY_HEADER_SIGNATURE.len())
            .any(|window| window == SONY_HEADER_SIGNATURE);
        if !has_signature {
            return Some(
                "Missing 'Sony Interactive Entertainment' signature in firmware header 0x00..0x40."
                    .to_string(),
            );
        }

        None
    }

    /// Validates that the image meets official Sony DualSense binary size and header requirements.
    pub fn validate_sony_header(&self) -> Result<()> {
        if let Some(issue) = self.sony_header_issue() {
            return Err(CoreError::InvalidDownloadedFirmware(issue));
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
        let expected_pid = expected_pid_for_target(&release.target_id);
        let pid_matches = match expected_pid {
            Some(pid) => actual_pid == pid,
            None => actual_pid == DEFAULT_PID || actual_pid == DUALSENSE_EDGE_PID,
        };
        if !pid_matches {
            return Err(CoreError::InvalidDownloadedFirmware(format!(
                "Embedded USB PID 0x{actual_pid:04x} does not match target {} (expected 0x{:04x})",
                release.target_id,
                expected_pid.unwrap_or(DEFAULT_PID),
            )));
        }

        Ok(())
    }

    /// Validates a local firmware image against a connected controller's hardware info and USB PID.
    pub fn validate_for_controller(
        &self,
        hardware: &ControllerHardwareInfo,
        product_id: u16,
    ) -> LocalImageValidation {
        let expected_target_id = hardware.supported_target_id.map(ToString::to_string);
        if let Some(issue) = self.sony_header_issue() {
            return LocalImageValidation {
                compatibility: LocalImageCompatibility::Incompatible,
                sony_header_valid: false,
                image_version: self.version(),
                embedded_pid: None,
                embedded_target_id: None,
                expected_pid: product_id,
                expected_target_id,
                mismatches: vec![issue],
                warnings: Vec::new(),
            };
        }

        let actual_pid = self.embedded_pid();
        let actual_target = self.embedded_target_id();
        let actual_target_hex = format!("{actual_target:04X}");
        let mut mismatches = Vec::new();
        let mut warnings = Vec::new();

        if actual_pid != product_id {
            mismatches.push(format!(
                "Embedded USB PID 0x{actual_pid:04x} does not match connected controller PID 0x{product_id:04x}."
            ));
        }

        if let Some(expected_for_image) = expected_pid_for_target(&actual_target_hex)
            && actual_pid != expected_for_image
        {
            mismatches.push(format!(
                "Image USB PID 0x{actual_pid:04x} is inconsistent with image target {actual_target_hex} (expected 0x{expected_for_image:04x})."
            ));
        }

        match hardware.supported_target_id {
            Some(expected_target_str) => {
                if let Ok(expected_target) = parse_target_id_hex(expected_target_str)
                    && actual_target != expected_target
                {
                    let target_desc = find_known_target(&actual_target_hex)
                        .map(|info| format!("{actual_target_hex} ({})", info.board_revisions))
                        .unwrap_or_else(|| actual_target_hex.clone());
                    mismatches.push(format!(
                        "Firmware target mismatch: image is for target {target_desc}, but connected controller is {} (target {expected_target_str}).",
                        hardware.board_name,
                    ));
                }
            }
            None => {
                warnings.push(format!(
                    "Connected controller ({}) has no official Sony target ID; image target {actual_target_hex} cannot be verified automatically.",
                    hardware.board_name,
                ));
            }
        }

        let compatibility = if !mismatches.is_empty() {
            LocalImageCompatibility::Incompatible
        } else if !warnings.is_empty() {
            LocalImageCompatibility::Warning
        } else {
            LocalImageCompatibility::Compatible
        };

        LocalImageValidation {
            compatibility,
            sony_header_valid: true,
            image_version: self.version(),
            embedded_pid: Some(actual_pid),
            embedded_target_id: Some(actual_target_hex),
            expected_pid: product_id,
            expected_target_id,
            mismatches,
            warnings,
        }
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

fn expected_pid_for_target(target_id: &str) -> Option<u16> {
    if target_id.eq_ignore_ascii_case("0044") {
        Some(DUALSENSE_EDGE_PID)
    } else if target_id.eq_ignore_ascii_case("0004")
        || target_id.eq_ignore_ascii_case("000B")
        || target_id.eq_ignore_ascii_case("000E")
    {
        Some(DEFAULT_PID)
    } else {
        None
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
