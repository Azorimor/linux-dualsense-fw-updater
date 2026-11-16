use crate::protocol::{
    StartUpdateStatusCode, UpdateCommand, VerifyUpdateStatusCode, WriteUpdateStatusCode,
};

/// Result alias using [`CoreError`].
pub type Result<T> = std::result::Result<T, CoreError>;

/// High-level phase identifier used in progress reporting and timeout diagnostics.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum UpdatePhase {
    Starting,
    Writing,
    Verifying,
    Finalizing,
    Completed,
}

impl UpdatePhase {
    /// Returns a human-readable lowercase phase label for messages.
    pub fn label(self) -> &'static str {
        match self {
            Self::Starting => "update initialization",
            Self::Writing => "firmware writing",
            Self::Verifying => "firmware verification",
            Self::Finalizing => "update finalization",
            Self::Completed => "completion",
        }
    }
}

/// Plain-English-friendly error type for DualSense HID, firmware validation, update, and catalog operations.
#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum CoreError {
    #[error("Could not communicate with the DualSense controller over USB.")]
    Hid(String),

    #[error("Could not read or access the firmware file.")]
    Io(String),

    #[error("No DualSense controller was detected over USB.")]
    DeviceNotFound { vid: u16, pid: u16 },

    #[error("The selected controller is no longer connected.")]
    DevicePathNotMatched(String),

    #[error("The firmware file is too small to read a firmware version.")]
    FirmwareImageTooSmall,

    #[error("The firmware file is too small to be a valid DualSense firmware image (minimum 256 bytes).")]
    FirmwareImageTooSmallForHeader,

    #[error("Firmware update header has an invalid length.")]
    InvalidUpdateStreamLength(usize),

    #[error("Firmware write block exceeds the controller limit of 32 KiB.")]
    UpdateImageTooLarge(usize),

    #[error("The controller returned an incomplete firmware information report.")]
    FirmwareInfoTooShort(usize),

    #[error("The controller returned a truncated firmware information payload.")]
    FirmwareInfoPayloadTooShort(usize),

    #[error("The controller returned an empty update status response.")]
    UpdateStatusEmpty,

    #[error("The controller returned a malformed update status response.")]
    UpdateStatusMalformed(usize),

    #[error("The controller responded with an unexpected update status command.")]
    UnexpectedUpdateStatusCommand {
        actual: UpdateCommand,
        expected: UpdateCommand,
    },

    #[error("{0}")]
    UpdateFailed(UpdateFailure),

    #[error("Timed out waiting for the controller during {}.", .phase.label())]
    PollTimeout { phase: UpdatePhase, attempts: usize },

    #[error("The operation was cancelled.")]
    Cancelled,

    #[error("Could not fetch firmware data from Sony's update server.")]
    CatalogHttp(String),

    #[error("Sony's firmware catalog response had an unexpected format.")]
    CatalogParse(String),

    #[error("No firmware release was found for controller target {0}.")]
    CatalogTargetNotFound(String),
}

impl CoreError {
    /// Returns the plain-English message suitable for display in a GUI.
    pub fn user_message(&self) -> String {
        self.to_string()
    }

    /// Returns detailed technical diagnostics for logging or expandable details views.
    pub fn technical_detail(&self) -> String {
        match self {
            Self::Hid(detail) => format!("HID error: {detail}"),
            Self::Io(detail) => format!("I/O error: {detail}"),
            Self::DeviceNotFound { vid, pid } => {
                format!("Device not found for VID:PID {vid:04x}:{pid:04x}")
            }
            Self::DevicePathNotMatched(path) => format!("No device path matched {path}"),
            Self::FirmwareImageTooSmall => {
                "Firmware image is smaller than 0x7A bytes required for version offset 0x78"
                    .to_string()
            }
            Self::FirmwareImageTooSmallForHeader => {
                "Firmware image must be at least 256 bytes".to_string()
            }
            Self::InvalidUpdateStreamLength(len) => {
                format!("StartUpdate header stream must be 256 bytes, got {len}")
            }
            Self::UpdateImageTooLarge(len) => {
                format!("WriteUpdateImage block must be <= 0x8000 bytes, got {len}")
            }
            Self::FirmwareInfoTooShort(len) => {
                format!("Firmware info report too short: {len} bytes")
            }
            Self::FirmwareInfoPayloadTooShort(len) => {
                format!("Firmware info payload too short: {len} bytes")
            }
            Self::UpdateStatusEmpty => "Update status report is empty".to_string(),
            Self::UpdateStatusMalformed(len) => {
                format!("Update status report malformed: {len} bytes")
            }
            Self::UnexpectedUpdateStatusCommand { actual, expected } => {
                format!("Unexpected update status command: {actual:?} (expected {expected:?})")
            }
            Self::UpdateFailed(failure) => failure.technical_detail(),
            Self::PollTimeout { phase, attempts } => {
                format!("Poll timeout during {phase:?} after {attempts} attempts")
            }
            Self::Cancelled => "Operation cancelled via CancellationToken".to_string(),
            Self::CatalogHttp(detail) => format!("Catalog HTTP error: {detail}"),
            Self::CatalogParse(detail) => format!("Catalog parse error: {detail}"),
            Self::CatalogTargetNotFound(target) => {
                format!("Catalog target not found: {target}")
            }
        }
    }
}

impl From<hidapi::HidError> for CoreError {
    fn from(err: hidapi::HidError) -> Self {
        Self::Hid(err.to_string())
    }
}

impl From<std::io::Error> for CoreError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

/// Controller-reported firmware update rejection or failure grouped by update command.
#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
pub enum UpdateFailure {
    #[error("{0}")]
    StartUpdate(StartUpdateError),
    #[error("{0}")]
    WriteUpdateImage(WriteUpdateImageError),
    #[error("{0}")]
    VerifyUpdateImage(VerifyUpdateImageError),
    #[error("{0}")]
    FinalizeUpdate(FinalizeUpdateError),
}

impl UpdateFailure {
    /// Returns the protocol status code name for diagnostics.
    pub fn code_name(self) -> &'static str {
        match self {
            Self::StartUpdate(err) => err.code_name(),
            Self::WriteUpdateImage(err) => err.code_name(),
            Self::VerifyUpdateImage(err) => err.code_name(),
            Self::FinalizeUpdate(err) => err.code_name(),
        }
    }

    /// Returns a concise technical representation for logs.
    pub fn technical_detail(self) -> String {
        match self {
            Self::StartUpdate(err) => format!("UpdateFailed(StartUpdate({}))", err.code_name()),
            Self::WriteUpdateImage(err) => {
                format!("UpdateFailed(WriteUpdateImage({}))", err.code_name())
            }
            Self::VerifyUpdateImage(err) => {
                format!("UpdateFailed(VerifyUpdateImage({}))", err.code_name())
            }
            Self::FinalizeUpdate(err) => {
                format!("UpdateFailed(FinalizeUpdate({}))", err.code_name())
            }
        }
    }
}

/// Failure codes returned during `StartUpdate`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
pub enum StartUpdateError {
    #[error("Firmware image header authentication failed.")]
    HeaderCmacCheckError,
    #[error("Firmware image is not an upgrade; downgrades are not allowed.")]
    HeaderVersionCheckError,
    #[error("Firmware image header capability info is invalid.")]
    HeaderCapabilityInfoError,
    #[error("Device failed to erase flash for the update.")]
    HeaderFlashEraseError,
    #[error("Device did not receive the firmware header.")]
    HeaderInfoNotReceived,
    #[error("Firmware image header parameters are invalid.")]
    HeaderCommonParamError,
    #[error("Firmware image header failed for an unknown reason.")]
    HeaderOtherError,
}

impl StartUpdateError {
    /// Maps a [`StartUpdateStatusCode`] to `Some(StartUpdateError)` if it represents a failure.
    pub fn from_status(code: StartUpdateStatusCode) -> Option<Self> {
        match code {
            StartUpdateStatusCode::Success
            | StartUpdateStatusCode::Processing
            | StartUpdateStatusCode::Retry => None,
            StartUpdateStatusCode::HeaderCmacCheckError => Some(Self::HeaderCmacCheckError),
            StartUpdateStatusCode::HeaderVersionCheckError => Some(Self::HeaderVersionCheckError),
            StartUpdateStatusCode::HeaderCapabilityInfoError => {
                Some(Self::HeaderCapabilityInfoError)
            }
            StartUpdateStatusCode::HeaderFlashEraseError => Some(Self::HeaderFlashEraseError),
            StartUpdateStatusCode::HeaderInfoNotReceived => Some(Self::HeaderInfoNotReceived),
            StartUpdateStatusCode::HeaderCommonParamError => Some(Self::HeaderCommonParamError),
            StartUpdateStatusCode::HeaderOtherError => Some(Self::HeaderOtherError),
        }
    }

    /// Returns the uppercase protocol status code name.
    pub fn code_name(self) -> &'static str {
        match self {
            Self::HeaderCmacCheckError => "HEADER_CMAC_CHECK_ERROR",
            Self::HeaderVersionCheckError => "HEADER_VERSION_CHECK_ERROR",
            Self::HeaderCapabilityInfoError => "HEADER_CAPABILITY_INFO_ERROR",
            Self::HeaderFlashEraseError => "HEADER_FLASH_ERASE_ERROR",
            Self::HeaderInfoNotReceived => "HEADER_INFO_NOT_RECEIVED",
            Self::HeaderCommonParamError => "HEADER_COMMON_PARAM_ERROR",
            Self::HeaderOtherError => "HEADER_OTHER_ERROR",
        }
    }
}

/// Failure codes returned during `WriteUpdateImage`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
pub enum WriteUpdateImageError {
    #[error("Device failed while writing the firmware image.")]
    WriteImageFlashWriteError,
    #[error("Firmware write was attempted before the update was started.")]
    WriteUpdateNotStarted,
    #[error("Firmware image parameters are invalid.")]
    WriteImageCommonParamError,
    #[error("Firmware image write failed for an unknown reason.")]
    WriteImageOtherError,
}

impl WriteUpdateImageError {
    /// Maps a [`WriteUpdateStatusCode`] to `Some(WriteUpdateImageError)` if it represents a failure.
    pub fn from_status(code: WriteUpdateStatusCode) -> Option<Self> {
        match code {
            WriteUpdateStatusCode::Success
            | WriteUpdateStatusCode::SendNext
            | WriteUpdateStatusCode::Retry
            | WriteUpdateStatusCode::AlsoRetry => None,
            WriteUpdateStatusCode::WriteImageFlashWriteError => {
                Some(Self::WriteImageFlashWriteError)
            }
            WriteUpdateStatusCode::WriteUpdateNotStarted => Some(Self::WriteUpdateNotStarted),
            WriteUpdateStatusCode::WriteImageCommonParamError => {
                Some(Self::WriteImageCommonParamError)
            }
            WriteUpdateStatusCode::WriteImageOtherError => Some(Self::WriteImageOtherError),
        }
    }

    /// Returns the uppercase protocol status code name.
    pub fn code_name(self) -> &'static str {
        match self {
            Self::WriteImageFlashWriteError => "WRITE_IMAGE_FLASH_WRITE_ERROR",
            Self::WriteUpdateNotStarted => "WRITE_UPDATE_NOT_STARTED",
            Self::WriteImageCommonParamError => "WRITE_IMAGE_COMMON_PARAM_ERROR",
            Self::WriteImageOtherError => "WRITE_IMAGE_OTHER_ERROR",
        }
    }
}

/// Failure codes returned during `VerifyUpdateImage`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
pub enum VerifyUpdateImageError {
    #[error("Firmware image header authentication failed during verification.")]
    VerifyHeaderCmacCheckError,
    #[error("Firmware image is not an upgrade; downgrades are not allowed.")]
    VerifyHeaderVersionCheckError,
    #[error("Firmware image capability info is invalid.")]
    VerifyCapabilityInfoError,
    #[error("Firmware image body authentication failed.")]
    VerifyFwBodyCmacCheckError,
    #[error("Firmware image parameters are invalid.")]
    VerifyCommonParamError,
    #[error("Firmware image verification failed for an unknown reason.")]
    VerifyOtherError,
}

impl VerifyUpdateImageError {
    /// Maps a [`VerifyUpdateStatusCode`] to `Some(VerifyUpdateImageError)` if it represents a failure.
    pub fn from_status(code: VerifyUpdateStatusCode) -> Option<Self> {
        match code {
            VerifyUpdateStatusCode::Success | VerifyUpdateStatusCode::KeepPolling => None,
            VerifyUpdateStatusCode::VerifyHeaderCmacCheckError => {
                Some(Self::VerifyHeaderCmacCheckError)
            }
            VerifyUpdateStatusCode::VerifyHeaderVersionCheckError => {
                Some(Self::VerifyHeaderVersionCheckError)
            }
            VerifyUpdateStatusCode::VerifyCapabilityInfoError => {
                Some(Self::VerifyCapabilityInfoError)
            }
            VerifyUpdateStatusCode::VerifyFwBodyCmacCheckError => {
                Some(Self::VerifyFwBodyCmacCheckError)
            }
            VerifyUpdateStatusCode::VerifyCommonParamError => Some(Self::VerifyCommonParamError),
            VerifyUpdateStatusCode::VerifyOtherError => Some(Self::VerifyOtherError),
        }
    }

    /// Returns the uppercase protocol status code name.
    pub fn code_name(self) -> &'static str {
        match self {
            Self::VerifyHeaderCmacCheckError => "VERIFY_HEADER_CMAC_CHECK_ERROR",
            Self::VerifyHeaderVersionCheckError => "VERIFY_HEADER_VERSION_CHECK_ERROR",
            Self::VerifyCapabilityInfoError => "VERIFY_CAPABILITY_INFO_ERROR",
            Self::VerifyFwBodyCmacCheckError => "VERIFY_FW_BODY_CMAC_CHECK_ERROR",
            Self::VerifyCommonParamError => "VERIFY_COMMON_PARAM_ERROR",
            Self::VerifyOtherError => "VERIFY_OTHER_ERROR",
        }
    }
}

/// Failure codes associated with `FinalizeUpdate`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
pub enum FinalizeUpdateError {
    #[error("Finalizing the firmware update failed for an unknown reason.")]
    FinalizeOtherError,
}

impl FinalizeUpdateError {
    /// Returns the uppercase protocol status code name.
    pub fn code_name(self) -> &'static str {
        match self {
            Self::FinalizeOtherError => "FINALIZE_OTHER_ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            VerifyUpdateImageError::from_status(
                VerifyUpdateStatusCode::VerifyHeaderVersionCheckError
            ),
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

        let io_err: CoreError =
            std::io::Error::new(std::io::ErrorKind::NotFound, "missing.bin").into();
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
                UpdateFailure::VerifyUpdateImage(
                    VerifyUpdateImageError::VerifyFwBodyCmacCheckError,
                ),
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
}
