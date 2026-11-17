// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT AND GPL-3.0-or-later

use crate::error::*;
use crate::protocol::{
    StartUpdateStatusCode, UpdateCommand, VerifyUpdateStatusCode, WriteUpdateStatusCode,
};

#[test]
fn start_update_error_maps_all_status_codes() {
    assert_eq!(
        StartUpdateError::from_status(StartUpdateStatusCode::Success),
        None
    );
    assert_eq!(
        StartUpdateError::from_status(StartUpdateStatusCode::Processing),
        None
    );
    assert_eq!(
        StartUpdateError::from_status(StartUpdateStatusCode::Retry),
        None
    );
    assert_eq!(
        StartUpdateError::from_status(StartUpdateStatusCode::HeaderCmacCheckError),
        Some(StartUpdateError::HeaderCmacCheckError)
    );
    assert_eq!(
        StartUpdateError::from_status(StartUpdateStatusCode::HeaderVersionCheckError),
        Some(StartUpdateError::HeaderVersionCheckError)
    );
    assert_eq!(
        StartUpdateError::from_status(StartUpdateStatusCode::HeaderCapabilityInfoError),
        Some(StartUpdateError::HeaderCapabilityInfoError)
    );
    assert_eq!(
        StartUpdateError::from_status(StartUpdateStatusCode::HeaderFlashEraseError),
        Some(StartUpdateError::HeaderFlashEraseError)
    );
    assert_eq!(
        StartUpdateError::from_status(StartUpdateStatusCode::HeaderInfoNotReceived),
        Some(StartUpdateError::HeaderInfoNotReceived)
    );
    assert_eq!(
        StartUpdateError::from_status(StartUpdateStatusCode::HeaderCommonParamError),
        Some(StartUpdateError::HeaderCommonParamError)
    );
    assert_eq!(
        StartUpdateError::from_status(StartUpdateStatusCode::HeaderOtherError),
        Some(StartUpdateError::HeaderOtherError)
    );
}

#[test]
fn write_update_error_maps_all_status_codes() {
    assert_eq!(
        WriteUpdateImageError::from_status(WriteUpdateStatusCode::Success),
        None
    );
    assert_eq!(
        WriteUpdateImageError::from_status(WriteUpdateStatusCode::SendNext),
        None
    );
    assert_eq!(
        WriteUpdateImageError::from_status(WriteUpdateStatusCode::Retry),
        None
    );
    assert_eq!(
        WriteUpdateImageError::from_status(WriteUpdateStatusCode::AlsoRetry),
        None
    );
    assert_eq!(
        WriteUpdateImageError::from_status(WriteUpdateStatusCode::WriteImageFlashWriteError),
        Some(WriteUpdateImageError::WriteImageFlashWriteError)
    );
    assert_eq!(
        WriteUpdateImageError::from_status(WriteUpdateStatusCode::WriteUpdateNotStarted),
        Some(WriteUpdateImageError::WriteUpdateNotStarted)
    );
    assert_eq!(
        WriteUpdateImageError::from_status(WriteUpdateStatusCode::WriteImageCommonParamError),
        Some(WriteUpdateImageError::WriteImageCommonParamError)
    );
    assert_eq!(
        WriteUpdateImageError::from_status(WriteUpdateStatusCode::WriteImageOtherError),
        Some(WriteUpdateImageError::WriteImageOtherError)
    );
}

#[test]
fn verify_update_error_maps_all_status_codes() {
    assert_eq!(
        VerifyUpdateImageError::from_status(VerifyUpdateStatusCode::Success),
        None
    );
    assert_eq!(
        VerifyUpdateImageError::from_status(VerifyUpdateStatusCode::KeepPolling),
        None
    );
    assert_eq!(
        VerifyUpdateImageError::from_status(VerifyUpdateStatusCode::VerifyHeaderCmacCheckError),
        Some(VerifyUpdateImageError::VerifyHeaderCmacCheckError)
    );
    assert_eq!(
        VerifyUpdateImageError::from_status(VerifyUpdateStatusCode::VerifyHeaderVersionCheckError),
        Some(VerifyUpdateImageError::VerifyHeaderVersionCheckError)
    );
    assert_eq!(
        VerifyUpdateImageError::from_status(VerifyUpdateStatusCode::VerifyCapabilityInfoError),
        Some(VerifyUpdateImageError::VerifyCapabilityInfoError)
    );
    assert_eq!(
        VerifyUpdateImageError::from_status(VerifyUpdateStatusCode::VerifyFwBodyCmacCheckError),
        Some(VerifyUpdateImageError::VerifyFwBodyCmacCheckError)
    );
    assert_eq!(
        VerifyUpdateImageError::from_status(VerifyUpdateStatusCode::VerifyCommonParamError),
        Some(VerifyUpdateImageError::VerifyCommonParamError)
    );
    assert_eq!(
        VerifyUpdateImageError::from_status(VerifyUpdateStatusCode::VerifyOtherError),
        Some(VerifyUpdateImageError::VerifyOtherError)
    );
}

#[test]
fn errors_expose_plain_english_user_messages_and_technical_details() {
    let downgrade = CoreError::UpdateFailed(UpdateFailure::StartUpdate(
        StartUpdateError::HeaderVersionCheckError,
    ));
    assert_eq!(
        downgrade.user_message(),
        "Firmware image is not an upgrade; downgrades are not allowed."
    );
    assert_eq!(
        downgrade.technical_detail(),
        "UpdateFailed(StartUpdate(HEADER_VERSION_CHECK_ERROR))"
    );

    let timeout = CoreError::PollTimeout {
        phase: UpdatePhase::Verifying,
        attempts: 500,
    };
    assert_eq!(
        timeout.user_message(),
        "Timed out waiting for the controller during firmware verification."
    );
    assert!(timeout.technical_detail().contains("500"));

    let io_err: CoreError = std::io::Error::new(std::io::ErrorKind::NotFound, "missing.bin").into();
    assert_eq!(
        io_err.user_message(),
        "Could not read or access the firmware file."
    );
    assert!(io_err.technical_detail().contains("missing.bin"));
}

#[test]
fn update_failure_and_finalize_error_code_names_and_details() {
    let cases = [
        (
            UpdateFailure::StartUpdate(StartUpdateError::HeaderCmacCheckError),
            "HEADER_CMAC_CHECK_ERROR",
        ),
        (
            UpdateFailure::WriteUpdateImage(WriteUpdateImageError::WriteImageFlashWriteError),
            "WRITE_IMAGE_FLASH_WRITE_ERROR",
        ),
        (
            UpdateFailure::VerifyUpdateImage(VerifyUpdateImageError::VerifyFwBodyCmacCheckError),
            "VERIFY_FW_BODY_CMAC_CHECK_ERROR",
        ),
        (
            UpdateFailure::FinalizeUpdate(FinalizeUpdateError::FinalizeOtherError),
            "FINALIZE_OTHER_ERROR",
        ),
    ];
    for (failure, expected_code) in cases {
        assert_eq!(failure.code_name(), expected_code);
        assert!(failure.technical_detail().contains(expected_code));
    }
}

#[test]
fn technical_detail_covers_all_core_error_variants() {
    let variants = [
        CoreError::Hid("broken pipe".into()),
        CoreError::Io("permission denied".into()),
        CoreError::DeviceNotFound {
            vid: 0x054c,
            pid: 0x0ce6,
        },
        CoreError::DevicePathNotMatched("/dev/hidraw9".into()),
        CoreError::FirmwareImageTooSmall,
        CoreError::FirmwareImageTooSmallForHeader,
        CoreError::InvalidUpdateStreamLength(128),
        CoreError::UpdateImageTooLarge(40000),
        CoreError::FirmwareInfoTooShort(10),
        CoreError::FirmwareInfoPayloadTooShort(30),
        CoreError::UpdateStatusEmpty,
        CoreError::UpdateStatusMalformed(3),
        CoreError::UnexpectedUpdateStatusCommand {
            actual: UpdateCommand::VerifyUpdateImage,
            expected: UpdateCommand::StartUpdate,
        },
        CoreError::Cancelled,
        CoreError::CatalogHttp("503".into()),
        CoreError::CatalogParse("bad json".into()),
        CoreError::CatalogTargetNotFound("000B".into()),
    ];

    for variant in variants {
        assert!(!variant.user_message().is_empty());
        assert!(!variant.technical_detail().is_empty());
    }
    assert_eq!(UpdatePhase::Starting.label(), "update initialization");
    assert_eq!(UpdatePhase::Writing.label(), "firmware writing");
    assert_eq!(UpdatePhase::Finalizing.label(), "update finalization");
    assert_eq!(UpdatePhase::Completed.label(), "completion");
}
