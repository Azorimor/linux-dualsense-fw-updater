// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_ascii_stops_at_first_nul_and_handles_non_utf8() {
        assert_eq!(decode_ascii(b"Jan 01 2026\0extra"), "Jan 01 2026");
        assert_eq!(decode_ascii(b"\0anything"), "");
        assert_eq!(decode_ascii(b"12:34:56"), "12:34:56");
        assert_eq!(decode_ascii(&[b'A', 0xFF, b'B', 0]), "A\u{FFFD}B");
    }

    #[test]
    fn firmware_info_formats_version_as_padded_hex() {
        let info = FirmwareInfo {
            build_date: "Jan 01 2026".to_string(),
            build_time: "00:00:00".to_string(),
            firmware_version: 0x0630,
            unknown: vec![],
            raw: vec![],
        };
        assert_eq!(info.formatted_version(), "0x0630");
    }

    #[test]
    fn update_command_roundtrip_and_unknown() {
        assert_eq!(UpdateCommand::from_int(0x00), UpdateCommand::StartUpdate);
        assert_eq!(
            UpdateCommand::from_int(0x01),
            UpdateCommand::WriteUpdateImage
        );
        assert_eq!(
            UpdateCommand::from_int(0x02),
            UpdateCommand::VerifyUpdateImage
        );
        assert_eq!(UpdateCommand::from_int(0x03), UpdateCommand::FinalizeUpdate);
        assert_eq!(UpdateCommand::from_int(0x04), UpdateCommand::Unknown);
        assert_eq!(UpdateCommand::from_int(0xFF), UpdateCommand::Unknown);
    }

    #[test]
    fn start_update_status_code_mapping_and_names() {
        let cases = [
            (0x00, StartUpdateStatusCode::Success, "SUCCESS"),
            (
                0x01,
                StartUpdateStatusCode::HeaderCmacCheckError,
                "HEADER_CMAC_CHECK_ERROR",
            ),
            (
                0x02,
                StartUpdateStatusCode::HeaderVersionCheckError,
                "HEADER_VERSION_CHECK_ERROR",
            ),
            (
                0x03,
                StartUpdateStatusCode::HeaderCapabilityInfoError,
                "HEADER_CAPABILITY_INFO_ERROR",
            ),
            (0x04, StartUpdateStatusCode::Processing, "PROCESSING"),
            (
                0x05,
                StartUpdateStatusCode::HeaderFlashEraseError,
                "HEADER_FLASH_ERASE_ERROR",
            ),
            (
                0x06,
                StartUpdateStatusCode::HeaderInfoNotReceived,
                "HEADER_INFO_NOT_RECEIVED",
            ),
            (0x10, StartUpdateStatusCode::Retry, "RETRY"),
            (
                0x11,
                StartUpdateStatusCode::HeaderCommonParamError,
                "HEADER_COMMON_PARAM_ERROR",
            ),
            (
                0x42,
                StartUpdateStatusCode::HeaderOtherError,
                "HEADER_OTHER_ERROR",
            ),
        ];
        for (raw, expected, name) in cases {
            let decoded = StartUpdateStatusCode::from_int(raw);
            assert_eq!(decoded, expected);
            assert_eq!(decoded.name(), name);
        }
    }

    #[test]
    fn write_update_status_code_mapping_and_names() {
        let cases = [
            (0x00, WriteUpdateStatusCode::Success, "SUCCESS"),
            (0x01, WriteUpdateStatusCode::Retry, "RETRY"),
            (
                0x02,
                WriteUpdateStatusCode::WriteImageFlashWriteError,
                "WRITE_IMAGE_FLASH_WRITE_ERROR",
            ),
            (0x03, WriteUpdateStatusCode::SendNext, "SEND_NEXT"),
            (
                0x04,
                WriteUpdateStatusCode::WriteUpdateNotStarted,
                "WRITE_UPDATE_NOT_STARTED",
            ),
            (0x10, WriteUpdateStatusCode::AlsoRetry, "ALSO_RETRY"),
            (
                0x11,
                WriteUpdateStatusCode::WriteImageCommonParamError,
                "WRITE_IMAGE_COMMON_PARAM_ERROR",
            ),
            (
                0x77,
                WriteUpdateStatusCode::WriteImageOtherError,
                "WRITE_IMAGE_OTHER_ERROR",
            ),
        ];
        for (raw, expected, name) in cases {
            let decoded = WriteUpdateStatusCode::from_int(raw);
            assert_eq!(decoded, expected);
            assert_eq!(decoded.name(), name);
        }
    }

    #[test]
    fn verify_update_status_code_mapping_and_names() {
        let cases = [
            (0x00, VerifyUpdateStatusCode::Success, "SUCCESS"),
            (0x10, VerifyUpdateStatusCode::KeepPolling, "KEEP_POLLING"),
            (
                0x01,
                VerifyUpdateStatusCode::VerifyHeaderCmacCheckError,
                "VERIFY_HEADER_CMAC_CHECK_ERROR",
            ),
            (
                0x02,
                VerifyUpdateStatusCode::VerifyHeaderVersionCheckError,
                "VERIFY_HEADER_VERSION_CHECK_ERROR",
            ),
            (
                0x03,
                VerifyUpdateStatusCode::VerifyCapabilityInfoError,
                "VERIFY_CAPABILITY_INFO_ERROR",
            ),
            (
                0x04,
                VerifyUpdateStatusCode::VerifyFwBodyCmacCheckError,
                "VERIFY_FW_BODY_CMAC_CHECK_ERROR",
            ),
            (
                0x11,
                VerifyUpdateStatusCode::VerifyCommonParamError,
                "VERIFY_COMMON_PARAM_ERROR",
            ),
            (
                0x99,
                VerifyUpdateStatusCode::VerifyOtherError,
                "VERIFY_OTHER_ERROR",
            ),
        ];
        for (raw, expected, name) in cases {
            let decoded = VerifyUpdateStatusCode::from_int(raw);
            assert_eq!(decoded, expected);
            assert_eq!(decoded.name(), name);
        }
    }

    #[test]
    fn encode_update_command_chunks_handles_empty_exact_and_multi_chunk() {
        let empty = encode_update_command_chunks(UpdateCommand::VerifyUpdateImage, &[]);
        assert_eq!(empty, vec![vec![REPORT_ID_UPDATE_COMMAND, 0x02, 0x00]]);

        let exact = vec![0xAAu8; MAX_HID_CHUNK_SIZE];
        let exact_packets = encode_update_command_chunks(UpdateCommand::WriteUpdateImage, &exact);
        assert_eq!(exact_packets.len(), 1);
        assert_eq!(
            exact_packets[0][..3],
            [REPORT_ID_UPDATE_COMMAND, 0x01, MAX_HID_CHUNK_SIZE as u8]
        );
        assert_eq!(&exact_packets[0][3..], exact.as_slice());

        let header = (0..256).map(|i| (i & 0xFF) as u8).collect::<Vec<_>>();
        let header_packets = encode_update_command_chunks(UpdateCommand::StartUpdate, &header);
        assert_eq!(header_packets.len(), 5);
        assert_eq!(header_packets[0].len(), 3 + 57);
        assert_eq!(header_packets[4].len(), 3 + (256 - 4 * 57));
        assert_eq!(header_packets[4][2], (256 - 4 * 57) as u8);
    }

    #[test]
    fn firmware_info_from_report_parses_64_and_65_byte_reports() {
        let mut report64 = vec![0u8; 64];
        report64[..11].copy_from_slice(b"Jan 15 2025");
        report64[12..20].copy_from_slice(b"10:20:30");
        report64[44] = 0x30;
        report64[45] = 0x06;

        let info64 = FirmwareInfo::from_report(report64).expect("valid 64-byte report");
        assert_eq!(info64.build_date, "Jan 15 2025");
        assert_eq!(info64.build_time, "10:20:30");
        assert_eq!(info64.firmware_version, 0x0630);
        assert_eq!(info64.unknown.len(), 44);

        let mut report65 = vec![0u8; 65];
        report65[0] = REPORT_ID_FIRMWARE_INFO;
        report65[1..12].copy_from_slice(b"Feb 01 2026");
        report65[13..21].copy_from_slice(b"01:02:03");
        report65[45] = 0x41;
        report65[46] = 0x06;

        let info65 = FirmwareInfo::from_report(report65).expect("valid 65-byte report");
        assert_eq!(info65.build_date, "Feb 01 2026");
        assert_eq!(info65.build_time, "01:02:03");
        assert_eq!(info65.firmware_version, 0x0641);
    }

    #[test]
    fn firmware_info_from_report_rejects_short_reports() {
        assert_eq!(
            FirmwareInfo::from_report(vec![0u8; 19]),
            Err(CoreError::FirmwareInfoTooShort(19))
        );
        assert_eq!(
            FirmwareInfo::from_report(vec![0u8; 46]),
            Err(CoreError::FirmwareInfoPayloadTooShort(46))
        );
    }

    #[test]
    fn update_status_from_report_parses_and_validates() {
        let status = UpdateStatus::from_report(vec![REPORT_ID_UPDATE_STATUS, 0x01, 0x03, 0x00])
            .expect("valid status");
        assert_eq!(status.report_id, REPORT_ID_UPDATE_STATUS);
        assert_eq!(status.command, UpdateCommand::WriteUpdateImage);
        assert_eq!(status.status_raw, 0x03);

        assert_eq!(
            UpdateStatus::from_report(vec![]),
            Err(CoreError::UpdateStatusEmpty)
        );
        assert_eq!(
            UpdateStatus::from_report(vec![0xF5, 0x01, 0x00]),
            Err(CoreError::UpdateStatusMalformed(3))
        );
        assert_eq!(
            UpdateStatus::from_report(vec![0xF4, 0x01, 0x00, 0x00]),
            Err(CoreError::UpdateStatusMalformed(4))
        );
    }

    #[test]
    fn firmware_info_parses_real_64_byte_report_without_leading_space_and_decodes_hw_fields() {
        let mut report = vec![0u8; 64];
        report[0] = REPORT_ID_FIRMWARE_INFO;
        report[1..12].copy_from_slice(b"Jul 18 2025");
        report[12..20].copy_from_slice(b"15:42:10");
        report[20..22].copy_from_slice(&0x0003u16.to_le_bytes()); // fw_type
        report[22..24].copy_from_slice(&0x000Bu16.to_le_bytes()); // fw_series (BDM-050)
        report[24..28].copy_from_slice(&0x0000_0811u32.to_le_bytes()); // hw_info
        report[28..32].copy_from_slice(&0x0001_0630u32.to_le_bytes()); // fw_version
        report[44..46].copy_from_slice(&0x0630u16.to_le_bytes()); // firmware_version

        let info = FirmwareInfo::from_report(report).expect("valid real-world 64-byte report");
        assert_eq!(info.build_date, "Jul 18 2025");
        assert_eq!(info.build_time, "15:42:10");
        assert_eq!(info.fw_type(), 0x0003);
        assert_eq!(info.fw_series(), 0x000B);
        assert_eq!(info.formatted_fw_series(), "0x000B");
        assert_eq!(info.hw_info(), 0x0000_0811);
        assert_eq!(info.fw_version(), 0x0001_0630);
        assert_eq!(info.firmware_version, 0x0630);
        assert_eq!(info.board_model(), BoardModel::Bdm050);
    }

    #[test]
    fn firmware_info_parses_65_byte_report_hw_fields_and_handles_empty_tail() {
        let mut report65 = vec![0u8; 65];
        report65[0] = REPORT_ID_FIRMWARE_INFO;
        report65[1..12].copy_from_slice(b"Sep 01 2025");
        report65[13..21].copy_from_slice(b"08:00:00");
        report65[21..23].copy_from_slice(&0x0001u16.to_le_bytes());
        report65[23..25].copy_from_slice(&0x000Eu16.to_le_bytes());
        report65[25..29].copy_from_slice(&0x0000_1100u32.to_le_bytes());
        report65[29..33].copy_from_slice(&0x0002_0641u32.to_le_bytes());
        report65[45..47].copy_from_slice(&0x0641u16.to_le_bytes());

        let info = FirmwareInfo::from_report(report65).expect("valid 65-byte report");
        assert_eq!(info.build_date, "Sep 01 2025");
        assert_eq!(info.fw_type(), 0x0001);
        assert_eq!(info.fw_series(), 0x000E);
        assert_eq!(info.hw_info(), 0x0000_1100);
        assert_eq!(info.fw_version(), 0x0002_0641);
        assert_eq!(info.firmware_version, 0x0641);
        assert_eq!(info.board_model(), BoardModel::Bdm060M);

        // Struct literal fallback (e.g. legacy mock with empty unknown and short/empty raw)
        let manual = FirmwareInfo {
            build_date: String::new(),
            build_time: String::new(),
            firmware_version: 0x0500,
            unknown: Vec::new(),
            raw: Vec::new(),
        };
        assert_eq!(manual.fw_type(), 0);
        assert_eq!(manual.fw_series(), 0);
        assert_eq!(manual.hw_info(), 0);
        assert_eq!(manual.fw_version(), 0);
        assert_eq!(manual.board_model(), BoardModel::Unknown);
    }

    #[test]
    fn board_model_decodes_all_revisions_and_targets() {
        let cases = [
            (0x0000_0300, BoardModel::Bdm010, "BDM-010", None),
            (0x0000_0411, BoardModel::Bdm020, "BDM-020", Some("0004")),
            (0x0000_0500, BoardModel::Bdm030, "BDM-030", Some("0004")),
            (0x0000_0600, BoardModel::Bdm040, "BDM-040", Some("0004")),
            (0x0000_0700, BoardModel::Bdm050, "BDM-050", Some("000B")),
            (0x0000_0811, BoardModel::Bdm050, "BDM-050", Some("000B")),
            (0x0000_0900, BoardModel::Bdm060R, "BDM-060R", Some("000E")),
            (0x0000_1100, BoardModel::Bdm060M, "BDM-060M", Some("000E")),
            (0x0000_1300, BoardModel::Bdm060X, "BDM-060X", Some("000E")),
            (0x0000_0000, BoardModel::Unknown, "Unknown", None),
            (0x0000_FF00, BoardModel::Unknown, "Unknown", None),
        ];

        for (hw_info, expected, label, target) in cases {
            let model = BoardModel::from_hw_info(hw_info);
            assert_eq!(model, expected);
            assert_eq!(model.as_str(), label);
            assert_eq!(model.to_string(), label);
            assert_eq!(model.sony_target_id(), target);
        }
    }

    #[test]
    fn decode_ds5_color_matches_all_known_codes_and_rejects_invalid_inputs() {
        let cases = [
            ("F12300ABCDEF12345", Some("White")),
            ("F12301ABCDEF12345", Some("Midnight Black")),
            ("F12302ABCDEF12345", Some("Cosmic Red")),
            ("F12303ABCDEF12345", Some("Nova Pink")),
            ("F12304ABCDEF12345", Some("Galactic Purple")),
            ("F12305ABCDEF12345", Some("Starlight Blue")),
            ("F12306ABCDEF12345", Some("Grey Camouflage")),
            ("F12307ABCDEF12345", Some("Volcanic Red")),
            ("F12308ABCDEF12345", Some("Sterling Silver")),
            ("F12309ABCDEF12345", Some("Cobalt Blue")),
            ("F12310ABCDEF12345", Some("Chroma Teal")),
            ("F12311ABCDEF12345", Some("Chroma Indigo")),
            ("F12312ABCDEF12345", Some("Chroma Pearl")),
            ("F12313ABCDEF12345", Some("HyperPop Techno Red")),
            ("F12314ABCDEF12345", Some("HyperPop Remix Green")),
            ("F12315ABCDEF12345", Some("HyperPop Rhythm Blue")),
            ("F12330ABCDEF12345", Some("30th Anniversary")),
            ("F123Z1ABCDEF12345", Some("God of War Ragnarok")),
            ("F123Z2ABCDEF12345", Some("Spider-Man 2")),
            ("F123Z3ABCDEF12345", Some("Astro Bot")),
            ("F123Z4ABCDEF12345", Some("Fortnite")),
            ("F123Z6ABCDEF12345", Some("The Last of Us")),
            ("F123ZAABCDEF12345", Some("God of War 20th Anniversary")),
            ("F123ZBABCDEF12345", Some("Icon Blue Limited Edition")),
            ("F123ZCABCDEF12345", Some("Ghost of Yotei Limited Edition")),
            ("F123ZDABCDEF12345", Some("Marathon Limited Edition")),
            ("F123ZEABCDEF12345", Some("Genshin Impact Limited Edition")),
            ("F123ZFABCDEF12345", Some("007 First Light Limited Edition")),
            ("f123zfabcdef12345", Some("007 First Light Limited Edition")),
            ("F12399ABCDEF12345", None),
            ("F123", None),
            ("", None),
            ("F123é0ABCDEF", None),
        ];

        for (serial, expected) in cases {
            assert_eq!(decode_ds5_color(serial), expected, "serial={serial}");
        }
    }
}
