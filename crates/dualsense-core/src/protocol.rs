// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT AND GPL-3.0-or-later

use crate::error::{CoreError, Result};

/// Default Sony USB vendor ID for DualSense controllers.
pub const DEFAULT_VID: u16 = 0x054c;

/// Default USB product ID for standard DualSense controllers.
pub const DEFAULT_PID: u16 = 0x0ce6;

/// USB product ID for DualSense Edge controllers.
pub const DUALSENSE_EDGE_PID: u16 = 0x0df2;

/// HID Feature Report ID for reading current controller firmware info.
pub const REPORT_ID_FIRMWARE_INFO: u8 = 0x20;

/// HID Feature Report ID for sending firmware update command chunks.
pub const REPORT_ID_UPDATE_COMMAND: u8 = 0xF4;

/// HID Feature Report ID for polling firmware update status.
pub const REPORT_ID_UPDATE_STATUS: u8 = 0xF5;

/// HID Feature Report ID for requesting controller system/serial metadata.
pub const REPORT_ID_SERIAL_COMMAND: u8 = 0x80;

/// HID Feature Report ID for reading controller system/serial metadata.
pub const REPORT_ID_SERIAL_RESPONSE: u8 = 0x81;

/// Expected byte size of the `0x20` firmware info feature report.
pub const FIRMWARE_INFO_REPORT_LEN: usize = 64;

/// Expected byte size of the `0x81` serial response feature report.
pub const SERIAL_REPORT_LEN: usize = 64;

/// Expected byte size of the `0xF5` update status feature report.
pub const UPDATE_STATUS_REPORT_LEN: usize = 4;

/// Byte offset of the little-endian `u16` USB product ID in an official `.bin` image.
pub const FIRMWARE_PID_OFFSET: usize = 0x62;

/// Byte offset of the little-endian `u16` firmware target/series in an official `.bin` image.
pub const FIRMWARE_TARGET_OFFSET: usize = 0x74;

/// Byte offset of the little-endian `u16` firmware version in a `.bin` image.
pub const FIRMWARE_VERSION_OFFSET: usize = 0x78;

/// Number of header bytes sent during `StartUpdate`.
pub const UPDATE_HEADER_LEN: usize = 256;

/// Maximum block size accepted per `WriteUpdateImage` outer block (`32 KiB`).
pub const WRITE_BLOCK_SIZE: usize = 0x8000;

/// Maximum payload bytes carried in a single `0xF4` HID feature report.
pub const MAX_HID_CHUNK_SIZE: usize = 0x39;

/// Decodes a NUL-terminated ASCII slice into a `String` using lossy UTF-8 conversion.
pub fn decode_ascii(data: &[u8]) -> String {
    let trimmed: Vec<u8> = data.iter().copied().take_while(|b| *b != 0).collect();
    String::from_utf8_lossy(&trimmed).to_string()
}

/// DualSense mainboard hardware revision decoded from `hw_info` (`(hw_info >> 8) & 0xff`).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BoardModel {
    Bdm010,
    Bdm020,
    Bdm030,
    Bdm040,
    Bdm050,
    Bdm060R,
    Bdm060M,
    Bdm060X,
    Unknown,
}

impl BoardModel {
    /// Decodes a [`BoardModel`] from the 32-bit `hw_info` field of Feature Report `0x20`.
    pub fn from_hw_info(hw_info: u32) -> Self {
        match (hw_info >> 8) & 0xff {
            0x03 => Self::Bdm010,
            0x04 => Self::Bdm020,
            0x05 => Self::Bdm030,
            0x06 => Self::Bdm040,
            0x07 | 0x08 => Self::Bdm050,
            0x09 => Self::Bdm060R,
            0x11 => Self::Bdm060M,
            0x13 => Self::Bdm060X,
            _ => Self::Unknown,
        }
    }

    /// Returns the human-readable board revision label (e.g. `"BDM-050"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bdm010 => "BDM-010",
            Self::Bdm020 => "BDM-020",
            Self::Bdm030 => "BDM-030",
            Self::Bdm040 => "BDM-040",
            Self::Bdm050 => "BDM-050",
            Self::Bdm060R => "BDM-060R",
            Self::Bdm060M => "BDM-060M",
            Self::Bdm060X => "BDM-060X",
            Self::Unknown => "Unknown",
        }
    }

    /// Returns the official Sony firmware target ID for standard DualSense boards, if supported.
    pub fn sony_target_id(self) -> Option<&'static str> {
        match self {
            Self::Bdm020 | Self::Bdm030 | Self::Bdm040 => Some("0004"),
            Self::Bdm050 => Some("000B"),
            Self::Bdm060R | Self::Bdm060M | Self::Bdm060X => Some("000E"),
            Self::Bdm010 | Self::Unknown => None,
        }
    }
}

impl std::fmt::Display for BoardModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Decodes a DualSense colorway name from characters `[4..6]` of the controller serial number.
pub fn decode_ds5_color(serial: &str) -> Option<&'static str> {
    if !serial.is_ascii() {
        return None;
    }
    let slice = serial.as_bytes().get(4..6)?;
    let mut code = [slice[0], slice[1]];
    code.make_ascii_uppercase();
    match &code {
        b"00" => Some("White"),
        b"01" => Some("Midnight Black"),
        b"02" => Some("Cosmic Red"),
        b"03" => Some("Nova Pink"),
        b"04" => Some("Galactic Purple"),
        b"05" => Some("Starlight Blue"),
        b"06" => Some("Grey Camouflage"),
        b"07" => Some("Volcanic Red"),
        b"08" => Some("Sterling Silver"),
        b"09" => Some("Cobalt Blue"),
        b"10" => Some("Chroma Teal"),
        b"11" => Some("Chroma Indigo"),
        b"12" => Some("Chroma Pearl"),
        b"13" => Some("HyperPop Techno Red"),
        b"14" => Some("HyperPop Remix Green"),
        b"15" => Some("HyperPop Rhythm Blue"),
        b"30" => Some("30th Anniversary"),
        b"Z1" => Some("God of War Ragnarok"),
        b"Z2" => Some("Spider-Man 2"),
        b"Z3" => Some("Astro Bot"),
        b"Z4" => Some("Fortnite"),
        b"Z6" => Some("The Last of Us"),
        b"ZA" => Some("God of War 20th Anniversary"),
        b"ZB" => Some("Icon Blue Limited Edition"),
        b"ZC" => Some("Ghost of Yotei Limited Edition"),
        b"ZD" => Some("Marathon Limited Edition"),
        b"ZE" => Some("Genshin Impact Limited Edition"),
        b"ZF" => Some("007 First Light Limited Edition"),
        _ => None,
    }
}

/// Firmware metadata returned by Feature Report `0x20`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareInfo {
    pub build_date: String,
    pub build_time: String,
    pub firmware_version: u16,
    pub unknown: Vec<u8>,
    pub raw: Vec<u8>,
}

fn split_firmware_info_payload(raw: &[u8]) -> (&[u8], bool) {
    if raw.len() > FIRMWARE_INFO_REPORT_LEN && raw[0] == REPORT_ID_FIRMWARE_INFO {
        (&raw[1..], false)
    } else {
        let has_leading_report_id =
            raw.len() == FIRMWARE_INFO_REPORT_LEN && raw[0] == REPORT_ID_FIRMWARE_INFO;
        (raw, has_leading_report_id)
    }
}

impl FirmwareInfo {
    /// Decodes a `0x20` HID feature report buffer into [`FirmwareInfo`].
    pub fn from_report(raw: Vec<u8>) -> Result<Self> {
        if raw.len() < 20 {
            return Err(CoreError::FirmwareInfoTooShort(raw.len()));
        }
        let (payload, has_leading_report_id) = split_firmware_info_payload(&raw);
        if payload.len() < 47 {
            return Err(CoreError::FirmwareInfoPayloadTooShort(payload.len()));
        }
        // 64-byte hidraw responses keep report ID 0x20 at payload[0], which would otherwise
        // decode as an ASCII space at the start of build_date.
        let build_date = if has_leading_report_id {
            decode_ascii(&payload[1..12])
        } else {
            decode_ascii(&payload[..12])
        };
        let build_time = decode_ascii(&payload[12..20]);
        let firmware_version = u16::from_le_bytes([payload[44], payload[45]]);
        let unknown = payload[20..].to_vec();
        Ok(Self {
            build_date,
            build_time,
            firmware_version,
            unknown,
            raw,
        })
    }

    fn metadata_tail(&self) -> Option<&[u8]> {
        if self.unknown.len() >= 12 {
            return Some(self.unknown.as_slice());
        }
        if self.raw.len() >= 32 {
            let (payload, _) = split_firmware_info_payload(&self.raw);
            if payload.len() >= 32 {
                return Some(&payload[20..]);
            }
        }
        None
    }

    /// Returns the firmware type (`payload[20..22]`, little-endian `u16`).
    pub fn fw_type(&self) -> u16 {
        self.metadata_tail()
            .map(|t| u16::from_le_bytes([t[0], t[1]]))
            .unwrap_or(0)
    }

    /// Returns the firmware series identifier (`payload[22..24]`, little-endian `u16`).
    pub fn fw_series(&self) -> u16 {
        self.metadata_tail()
            .map(|t| u16::from_le_bytes([t[2], t[3]]))
            .unwrap_or(0)
    }

    /// Returns the hardware info word (`payload[24..28]`, little-endian `u32`).
    pub fn hw_info(&self) -> u32 {
        self.metadata_tail()
            .map(|t| u32::from_le_bytes([t[4], t[5], t[6], t[7]]))
            .unwrap_or(0)
    }

    /// Returns the full firmware version word (`payload[28..32]`, little-endian `u32`).
    pub fn fw_version(&self) -> u32 {
        self.metadata_tail()
            .map(|t| u32::from_le_bytes([t[8], t[9], t[10], t[11]]))
            .unwrap_or(0)
    }

    /// Returns the decoded [`BoardModel`] from [`FirmwareInfo::hw_info`].
    pub fn board_model(&self) -> BoardModel {
        BoardModel::from_hw_info(self.hw_info())
    }

    /// Returns the firmware series formatted as `0xXXXX`.
    pub fn formatted_fw_series(&self) -> String {
        format!("0x{:04X}", self.fw_series())
    }

    /// Returns the firmware version formatted as `0xXXXX`.
    pub fn formatted_version(&self) -> String {
        format!("0x{:04x}", self.firmware_version)
    }
}

/// Commands sent inside `0xF4` update feature reports and echoed in `0xF5` status reports.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum UpdateCommand {
    StartUpdate = 0x00,
    WriteUpdateImage = 0x01,
    VerifyUpdateImage = 0x02,
    FinalizeUpdate = 0x03,
    Unknown = 0xFF,
}

impl UpdateCommand {
    /// Converts a raw protocol byte into an [`UpdateCommand`].
    pub fn from_int(value: u8) -> Self {
        match value {
            0x00 => Self::StartUpdate,
            0x01 => Self::WriteUpdateImage,
            0x02 => Self::VerifyUpdateImage,
            0x03 => Self::FinalizeUpdate,
            _ => Self::Unknown,
        }
    }
}

/// Status codes returned in `0xF5` after [`UpdateCommand::StartUpdate`].
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum StartUpdateStatusCode {
    Success = 0x00,
    HeaderCmacCheckError = 0x01,
    HeaderVersionCheckError = 0x02,
    HeaderCapabilityInfoError = 0x03,
    Processing = 0x04,
    HeaderFlashEraseError = 0x05,
    HeaderInfoNotReceived = 0x06,
    Retry = 0x10,
    HeaderCommonParamError = 0x11,
    HeaderOtherError = 0xFF,
}

impl StartUpdateStatusCode {
    /// Converts a raw protocol byte into a [`StartUpdateStatusCode`].
    pub fn from_int(value: u8) -> Self {
        match value {
            0x00 => Self::Success,
            0x01 => Self::HeaderCmacCheckError,
            0x02 => Self::HeaderVersionCheckError,
            0x03 => Self::HeaderCapabilityInfoError,
            0x04 => Self::Processing,
            0x05 => Self::HeaderFlashEraseError,
            0x06 => Self::HeaderInfoNotReceived,
            0x10 => Self::Retry,
            0x11 => Self::HeaderCommonParamError,
            _ => Self::HeaderOtherError,
        }
    }

    /// Returns the uppercase protocol constant name for logs and diagnostics.
    pub fn name(self) -> &'static str {
        match self {
            Self::Success => "SUCCESS",
            Self::HeaderCmacCheckError => "HEADER_CMAC_CHECK_ERROR",
            Self::HeaderVersionCheckError => "HEADER_VERSION_CHECK_ERROR",
            Self::HeaderCapabilityInfoError => "HEADER_CAPABILITY_INFO_ERROR",
            Self::Processing => "PROCESSING",
            Self::HeaderFlashEraseError => "HEADER_FLASH_ERASE_ERROR",
            Self::HeaderInfoNotReceived => "HEADER_INFO_NOT_RECEIVED",
            Self::Retry => "RETRY",
            Self::HeaderCommonParamError => "HEADER_COMMON_PARAM_ERROR",
            Self::HeaderOtherError => "HEADER_OTHER_ERROR",
        }
    }
}

/// Status codes returned in `0xF5` after [`UpdateCommand::WriteUpdateImage`].
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum WriteUpdateStatusCode {
    Success = 0x00,
    Retry = 0x01,
    WriteImageFlashWriteError = 0x02,
    SendNext = 0x03,
    WriteUpdateNotStarted = 0x04,
    AlsoRetry = 0x10,
    WriteImageCommonParamError = 0x11,
    WriteImageOtherError = 0xFF,
}

impl WriteUpdateStatusCode {
    /// Converts a raw protocol byte into a [`WriteUpdateStatusCode`].
    pub fn from_int(value: u8) -> Self {
        match value {
            0x00 => Self::Success,
            0x01 => Self::Retry,
            0x02 => Self::WriteImageFlashWriteError,
            0x03 => Self::SendNext,
            0x04 => Self::WriteUpdateNotStarted,
            0x10 => Self::AlsoRetry,
            0x11 => Self::WriteImageCommonParamError,
            _ => Self::WriteImageOtherError,
        }
    }

    /// Returns the uppercase protocol constant name for logs and diagnostics.
    pub fn name(self) -> &'static str {
        match self {
            Self::Success => "SUCCESS",
            Self::Retry => "RETRY",
            Self::WriteImageFlashWriteError => "WRITE_IMAGE_FLASH_WRITE_ERROR",
            Self::SendNext => "SEND_NEXT",
            Self::WriteUpdateNotStarted => "WRITE_UPDATE_NOT_STARTED",
            Self::AlsoRetry => "ALSO_RETRY",
            Self::WriteImageCommonParamError => "WRITE_IMAGE_COMMON_PARAM_ERROR",
            Self::WriteImageOtherError => "WRITE_IMAGE_OTHER_ERROR",
        }
    }
}

/// Status codes returned in `0xF5` after [`UpdateCommand::VerifyUpdateImage`].
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum VerifyUpdateStatusCode {
    Success = 0x00,
    KeepPolling = 0x10,
    VerifyHeaderCmacCheckError = 0x01,
    VerifyHeaderVersionCheckError = 0x02,
    VerifyCapabilityInfoError = 0x03,
    VerifyFwBodyCmacCheckError = 0x04,
    VerifyCommonParamError = 0x11,
    VerifyOtherError = 0xFF,
}

impl VerifyUpdateStatusCode {
    /// Converts a raw protocol byte into a [`VerifyUpdateStatusCode`].
    pub fn from_int(value: u8) -> Self {
        match value {
            0x00 => Self::Success,
            0x10 => Self::KeepPolling,
            0x01 => Self::VerifyHeaderCmacCheckError,
            0x02 => Self::VerifyHeaderVersionCheckError,
            0x03 => Self::VerifyCapabilityInfoError,
            0x04 => Self::VerifyFwBodyCmacCheckError,
            0x11 => Self::VerifyCommonParamError,
            _ => Self::VerifyOtherError,
        }
    }

    /// Returns the uppercase protocol constant name for logs and diagnostics.
    pub fn name(self) -> &'static str {
        match self {
            Self::Success => "SUCCESS",
            Self::KeepPolling => "KEEP_POLLING",
            Self::VerifyHeaderCmacCheckError => "VERIFY_HEADER_CMAC_CHECK_ERROR",
            Self::VerifyHeaderVersionCheckError => "VERIFY_HEADER_VERSION_CHECK_ERROR",
            Self::VerifyCapabilityInfoError => "VERIFY_CAPABILITY_INFO_ERROR",
            Self::VerifyFwBodyCmacCheckError => "VERIFY_FW_BODY_CMAC_CHECK_ERROR",
            Self::VerifyCommonParamError => "VERIFY_COMMON_PARAM_ERROR",
            Self::VerifyOtherError => "VERIFY_OTHER_ERROR",
        }
    }
}

/// Decoded `0xF5` update status feature report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateStatus {
    pub report_id: u8,
    pub command: UpdateCommand,
    pub status_raw: u8,
    pub raw: Vec<u8>,
}

impl UpdateStatus {
    /// Parses a `0xF5` update status feature report buffer.
    pub fn from_report(raw: Vec<u8>) -> Result<Self> {
        if raw.is_empty() {
            return Err(CoreError::UpdateStatusEmpty);
        }
        if raw[0] != REPORT_ID_UPDATE_STATUS || raw.len() != UPDATE_STATUS_REPORT_LEN {
            return Err(CoreError::UpdateStatusMalformed(raw.len()));
        }
        let command = UpdateCommand::from_int(raw[1]);
        Ok(Self {
            report_id: raw[0],
            command,
            status_raw: raw[2],
            raw,
        })
    }
}

/// Encodes a single `0xF4` feature report packet from an [`UpdateCommand`] and a chunk (`<= 0x39` bytes).
pub fn encode_update_report(command: UpdateCommand, chunk: &[u8]) -> Vec<u8> {
    debug_assert!(chunk.len() <= MAX_HID_CHUNK_SIZE);
    let mut packet = Vec::with_capacity(3 + chunk.len());
    packet.push(REPORT_ID_UPDATE_COMMAND);
    packet.push(command as u8);
    packet.push(chunk.len() as u8);
    packet.extend_from_slice(chunk);
    packet
}

/// Splits `payload` into `0xF4` feature reports of at most `0x39` bytes each.
///
/// An empty payload produces a single `[0xF4, command, 0x00]` report.
pub fn encode_update_command_chunks(command: UpdateCommand, payload: &[u8]) -> Vec<Vec<u8>> {
    if payload.is_empty() {
        return vec![encode_update_report(command, &[])];
    }
    payload
        .chunks(MAX_HID_CHUNK_SIZE)
        .map(|chunk| encode_update_report(command, chunk))
        .collect()
}
