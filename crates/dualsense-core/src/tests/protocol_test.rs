// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT

use crate::error::CoreError;
use crate::protocol::*;

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
