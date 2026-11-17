// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use dualsense_core::{
    CoreError, FinalizeUpdateError, StartUpdateError, UpdateCommand, UpdateFailure, UpdatePhase,
    VerifyUpdateImageError, WriteUpdateImageError,
};

use crate::error_map::{FriendlyError, map_core_error};

fn assert_user_friendly(err: &FriendlyError) {
    assert!(!err.error_title.is_empty());
    assert!(!err.error_message.is_empty());
    assert!(!err.recovery_hint.is_empty());
    assert!(!err.technical_detail.is_empty());

    for field in [&err.error_title, &err.error_message, &err.recovery_hint] {
        assert!(
            !field.contains("HEADER_")
                && !field.contains("WRITE_")
                && !field.contains("VERIFY_")
                && !field.contains("HidError")
                && !field.contains("stack backtrace")
                && !field.contains("UpdateFailed("),
            "leaked technical string into user-facing text: {field}"
        );
    }
}

#[test]
fn maps_every_core_error_variant_to_plain_english() {
    let cases = vec![
        CoreError::Hid("Permission denied opening /dev/hidraw0".to_string()),
        CoreError::Hid("device disconnected".to_string()),
        CoreError::Io("No such file".to_string()),
        CoreError::DeviceNotFound {
            vid: 0x054c,
            pid: 0x0ce6,
        },
        CoreError::DevicePathNotMatched("/dev/hidraw1".to_string()),
        CoreError::FirmwareImageTooSmall,
        CoreError::FirmwareImageTooSmallForHeader,
        CoreError::InvalidUpdateStreamLength(12),
        CoreError::UpdateImageTooLarge(99999),
        CoreError::FirmwareInfoTooShort(5),
        CoreError::FirmwareInfoPayloadTooShort(12),
        CoreError::UpdateStatusEmpty,
        CoreError::UpdateStatusMalformed(2),
        CoreError::UnexpectedUpdateStatusCommand {
            actual: UpdateCommand::Unknown,
            expected: UpdateCommand::StartUpdate,
        },
        CoreError::PollTimeout {
            phase: UpdatePhase::Verifying,
            attempts: 2000,
        },
        CoreError::Cancelled,
        CoreError::CatalogHttp("The connection to Sony's update server timed out.".to_string()),
        CoreError::CatalogHttp("".to_string()),
        CoreError::CatalogParse("bad json".to_string()),
        CoreError::CatalogTargetNotFound("0099".to_string()),
        CoreError::InvalidDownloadedFirmware("Missing Sony signature".to_string()),
        CoreError::FirmwareTargetMismatch {
            expected: "0004".to_string(),
            actual: "000B".to_string(),
        },
        CoreError::FirmwareVersionMismatch {
            expected: 0x0630,
            actual: 0x0520,
        },
    ];

    for err in &cases {
        let friendly = map_core_error(err);
        assert_user_friendly(&friendly);
    }

    let invalid_dl = map_core_error(&CoreError::InvalidDownloadedFirmware(
        "truncated".to_string(),
    ));
    assert!(invalid_dl.error_title.contains("invalid"));
    assert!(invalid_dl.recovery_hint.contains("try downloading again"));

    let target_mismatch = map_core_error(&CoreError::FirmwareTargetMismatch {
        expected: "0004".to_string(),
        actual: "000B".to_string(),
    });
    assert!(target_mismatch.error_message.contains("000B"));
    assert!(target_mismatch.error_message.contains("0004"));

    let version_mismatch = map_core_error(&CoreError::FirmwareVersionMismatch {
        expected: 0x0630,
        actual: 0x0520,
    });
    assert!(version_mismatch.error_message.contains("0x0630"));
    assert!(version_mismatch.error_message.contains("0x0520"));

    let perm = map_core_error(&CoreError::Hid("Permission denied".to_string()));
    assert!(perm.error_title.contains("permission"));
    assert!(perm.recovery_hint.contains("udev"));

    let timeout = map_core_error(&CoreError::PollTimeout {
        phase: UpdatePhase::Verifying,
        attempts: 50,
    });
    assert!(timeout.error_message.contains("firmware verification"));
}

#[test]
fn maps_every_update_failure_variant_to_plain_english() {
    let failures = [
        UpdateFailure::StartUpdate(StartUpdateError::HeaderCmacCheckError),
        UpdateFailure::StartUpdate(StartUpdateError::HeaderVersionCheckError),
        UpdateFailure::StartUpdate(StartUpdateError::HeaderCapabilityInfoError),
        UpdateFailure::StartUpdate(StartUpdateError::HeaderFlashEraseError),
        UpdateFailure::StartUpdate(StartUpdateError::HeaderInfoNotReceived),
        UpdateFailure::StartUpdate(StartUpdateError::HeaderCommonParamError),
        UpdateFailure::StartUpdate(StartUpdateError::HeaderOtherError),
        UpdateFailure::WriteUpdateImage(WriteUpdateImageError::WriteImageFlashWriteError),
        UpdateFailure::WriteUpdateImage(WriteUpdateImageError::WriteUpdateNotStarted),
        UpdateFailure::WriteUpdateImage(WriteUpdateImageError::WriteImageCommonParamError),
        UpdateFailure::WriteUpdateImage(WriteUpdateImageError::WriteImageOtherError),
        UpdateFailure::VerifyUpdateImage(VerifyUpdateImageError::VerifyHeaderCmacCheckError),
        UpdateFailure::VerifyUpdateImage(VerifyUpdateImageError::VerifyHeaderVersionCheckError),
        UpdateFailure::VerifyUpdateImage(VerifyUpdateImageError::VerifyCapabilityInfoError),
        UpdateFailure::VerifyUpdateImage(VerifyUpdateImageError::VerifyFwBodyCmacCheckError),
        UpdateFailure::VerifyUpdateImage(VerifyUpdateImageError::VerifyCommonParamError),
        UpdateFailure::VerifyUpdateImage(VerifyUpdateImageError::VerifyOtherError),
        UpdateFailure::FinalizeUpdate(FinalizeUpdateError::FinalizeOtherError),
    ];

    for failure in failures {
        let friendly = map_core_error(&CoreError::UpdateFailed(failure));
        assert_user_friendly(&friendly);
    }

    let downgrade = map_core_error(&CoreError::UpdateFailed(UpdateFailure::StartUpdate(
        StartUpdateError::HeaderVersionCheckError,
    )));
    assert!(downgrade.error_message.contains("already running"));
}

#[test]
fn sanitizes_technical_catalog_http_messages() {
    let leaked = map_core_error(&CoreError::CatalogHttp(
        "ureq::Error::Io stack backtrace".to_string(),
    ));
    assert_eq!(
        leaked.error_message,
        "Could not download firmware data from Sony's update server."
    );
}
