// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT AND GPL-3.0-or-later

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

    #[error(
        "The firmware file is too small to be a valid DualSense firmware image (minimum 256 bytes)."
    )]
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

    #[error(
        "The downloaded firmware file is corrupted or not a valid Sony DualSense firmware image."
    )]
    InvalidDownloadedFirmware(String),

    #[error("The downloaded firmware is for controller target {actual}, expected {expected}.")]
    FirmwareTargetMismatch { expected: String, actual: String },

    #[error(
        "The downloaded firmware version (0x{actual:04x}) does not match the expected release (0x{expected:04x})."
    )]
    FirmwareVersionMismatch { expected: u16, actual: u16 },
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
            Self::InvalidDownloadedFirmware(detail) => {
                format!("Invalid downloaded firmware: {detail}")
            }
            Self::FirmwareTargetMismatch { expected, actual } => {
                format!("Firmware target mismatch: expected {expected}, got {actual}")
            }
            Self::FirmwareVersionMismatch { expected, actual } => {
                format!("Firmware version mismatch: expected 0x{expected:04x}, got 0x{actual:04x}")
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
