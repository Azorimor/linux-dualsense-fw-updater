// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use dualsense_core::{
    CoreError, FinalizeUpdateError, StartUpdateError, UpdateFailure, VerifyUpdateImageError,
    WriteUpdateImageError,
};

/// Plain-English error presentation suitable for non-technical desktop users.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendlyError {
    pub error_title: String,
    pub error_message: String,
    pub recovery_hint: String,
    pub technical_detail: String,
}

/// Maps any [`CoreError`] into a non-technical [`FriendlyError`] with actionable recovery steps.
pub fn map_core_error(err: &CoreError) -> FriendlyError {
    let technical_detail = err.technical_detail();
    match err {
        CoreError::Hid(detail) => map_hid_error(detail, technical_detail),
        CoreError::Io(_) => FriendlyError {
            error_title: "Could not read firmware file".to_string(),
            error_message:
                "The selected firmware file could not be opened or read from your computer."
                    .to_string(),
            recovery_hint:
                "Check that the file still exists and is readable, then select the firmware file again."
                    .to_string(),
            technical_detail,
        },
        CoreError::DeviceNotFound { .. } => FriendlyError {
            error_title: "No DualSense controller found".to_string(),
            error_message: "No Sony DualSense controller was detected over USB.".to_string(),
            recovery_hint:
                "Plug your controller directly into your PC using a USB data cable (updates cannot run over Bluetooth), then click Scan Again."
                    .to_string(),
            technical_detail,
        },
        CoreError::DevicePathNotMatched(_) => FriendlyError {
            error_title: "Controller disconnected".to_string(),
            error_message: "The selected DualSense controller is no longer connected.".to_string(),
            recovery_hint:
                "Reconnect your controller with a USB cable and click Scan Again to detect it."
                    .to_string(),
            technical_detail,
        },
        CoreError::FirmwareImageTooSmall
        | CoreError::FirmwareImageTooSmallForHeader
        | CoreError::InvalidUpdateStreamLength(_) => FriendlyError {
            error_title: "Invalid firmware file".to_string(),
            error_message: "The selected file is not a valid DualSense firmware image.".to_string(),
            recovery_hint:
                "Choose a complete DualSense firmware .bin file, or download the official update from Sony instead."
                    .to_string(),
            technical_detail,
        },
        CoreError::UpdateImageTooLarge(_) => FriendlyError {
            error_title: "Firmware file exceeds transfer limit".to_string(),
            error_message: "Part of the firmware image is larger than the controller supports."
                .to_string(),
            recovery_hint:
                "Use an official Sony firmware file matching your controller model and try again."
                    .to_string(),
            technical_detail,
        },
        CoreError::FirmwareInfoTooShort(_) | CoreError::FirmwareInfoPayloadTooShort(_) => {
            FriendlyError {
                error_title: "Could not read controller details".to_string(),
                error_message:
                    "The controller returned an incomplete firmware information report.".to_string(),
                recovery_hint:
                    "Unplug the USB cable, wait a few seconds, plug it back directly into your PC, and click Scan Again."
                        .to_string(),
                technical_detail,
            }
        }
        CoreError::UpdateStatusEmpty
        | CoreError::UpdateStatusMalformed(_)
        | CoreError::UnexpectedUpdateStatusCommand { .. } => FriendlyError {
            error_title: "Unexpected controller response".to_string(),
            error_message:
                "The controller replied with an unexpected status during the firmware update."
                    .to_string(),
            recovery_hint:
                "Disconnect and reconnect the USB cable firmly, then check your controller's firmware version before trying again."
                    .to_string(),
            technical_detail,
        },
        CoreError::UpdateFailed(failure) => map_update_failure(*failure, technical_detail),
        CoreError::PollTimeout { phase, .. } => FriendlyError {
            error_title: "Controller took too long to respond".to_string(),
            error_message: format!("The controller stopped responding during {}.", phase.label()),
            recovery_hint:
                "Unplug the USB cable, plug the controller directly into a PC USB port (avoid unpowered hubs), and scan again."
                    .to_string(),
            technical_detail,
        },
        CoreError::Cancelled => FriendlyError {
            error_title: "Operation cancelled".to_string(),
            error_message: "The operation was cancelled before it finished.".to_string(),
            recovery_hint: "You can review your selection and try again whenever you are ready."
                .to_string(),
            technical_detail,
        },
        CoreError::CatalogHttp(message) => FriendlyError {
            error_title: "Could not reach Sony's update server".to_string(),
            error_message: sanitize_network_message(message),
            recovery_hint:
                "Check your internet connection and click Check for Updates again, or choose a local firmware .bin file."
                    .to_string(),
            technical_detail,
        },
        CoreError::CatalogParse(_) => FriendlyError {
            error_title: "Could not read Sony's update list".to_string(),
            error_message:
                "Sony's update server returned firmware catalog information in an unexpected format."
                    .to_string(),
            recovery_hint:
                "Try checking for updates again later, or install from a local firmware .bin file."
                    .to_string(),
            technical_detail,
        },
        CoreError::CatalogTargetNotFound(_) => FriendlyError {
            error_title: "No matching update found".to_string(),
            error_message:
                "Sony's update catalog did not list a firmware release for the selected controller model."
                    .to_string(),
            recovery_hint:
                "Make sure you selected the right DualSense model revision, or use a local firmware .bin file."
                    .to_string(),
            technical_detail,
        },
    }
}

fn map_hid_error(detail: &str, technical_detail: String) -> FriendlyError {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("permission")
        || lower.contains("access")
        || lower.contains("eacces")
        || lower.contains("eperm")
    {
        return FriendlyError {
            error_title: "USB permission required".to_string(),
            error_message:
                "Linux blocked access to the DualSense USB device because your user account lacks HID permissions."
                    .to_string(),
            recovery_hint:
                "Install DualSense udev rules (allowing user access to Sony USB HID devices), unplug and replug the controller, then click Scan Again."
                    .to_string(),
            technical_detail,
        };
    }

    FriendlyError {
        error_title: "Could not communicate with controller".to_string(),
        error_message:
            "The USB connection to your DualSense controller was interrupted or could not be opened."
                .to_string(),
        recovery_hint:
            "Use a reliable USB data cable plugged directly into your PC (not Bluetooth), unplug and replug the controller, and click Scan Again."
                .to_string(),
        technical_detail,
    }
}

fn map_update_failure(failure: UpdateFailure, technical_detail: String) -> FriendlyError {
    match failure {
        UpdateFailure::StartUpdate(StartUpdateError::HeaderVersionCheckError)
        | UpdateFailure::VerifyUpdateImage(VerifyUpdateImageError::VerifyHeaderVersionCheckError) => {
            FriendlyError {
                error_title: "Controller already has this or newer firmware".to_string(),
                error_message:
                    "The controller declined the update because it is already running this firmware version or newer. DualSense firmware cannot be downgraded."
                        .to_string(),
                recovery_hint:
                    "Your controller is likely already up to date. You can rescan it to verify its current version, or select a newer firmware image."
                        .to_string(),
                technical_detail,
            }
        }
        UpdateFailure::StartUpdate(StartUpdateError::HeaderCmacCheckError)
        | UpdateFailure::VerifyUpdateImage(
            VerifyUpdateImageError::VerifyHeaderCmacCheckError
            | VerifyUpdateImageError::VerifyFwBodyCmacCheckError,
        ) => FriendlyError {
            error_title: "Firmware verification check failed".to_string(),
            error_message:
                "The controller rejected the firmware because its authenticity check did not pass."
                    .to_string(),
            recovery_hint:
                "The firmware file may be corrupted or meant for a different controller revision. Re-download the official update for your exact DualSense model and try again."
                    .to_string(),
            technical_detail,
        },
        UpdateFailure::StartUpdate(StartUpdateError::HeaderCapabilityInfoError)
        | UpdateFailure::VerifyUpdateImage(VerifyUpdateImageError::VerifyCapabilityInfoError) => {
            FriendlyError {
                error_title: "Firmware does not match this controller".to_string(),
                error_message:
                    "This firmware update is intended for a different DualSense hardware revision."
                        .to_string(),
                recovery_hint:
                    "Check the model revision on the back of your controller (or select the right target option) and try again."
                        .to_string(),
                technical_detail,
            }
        }
        UpdateFailure::StartUpdate(StartUpdateError::HeaderFlashEraseError)
        | UpdateFailure::WriteUpdateImage(WriteUpdateImageError::WriteImageFlashWriteError) => {
            FriendlyError {
                error_title: "Controller could not save firmware".to_string(),
                error_message:
                    "The controller reported a problem while erasing or writing its internal firmware memory."
                        .to_string(),
                recovery_hint:
                    "Unplug the USB cable, plug the controller directly into your PC, and scan again to check its firmware status."
                        .to_string(),
                technical_detail,
            }
        }
        UpdateFailure::StartUpdate(StartUpdateError::HeaderInfoNotReceived)
        | UpdateFailure::WriteUpdateImage(WriteUpdateImageError::WriteUpdateNotStarted) => {
            FriendlyError {
                error_title: "Update could not start properly".to_string(),
                error_message: "The controller was not ready to receive the firmware data."
                    .to_string(),
                recovery_hint:
                    "Unplug and reconnect the controller over USB, then start the update again from the beginning."
                        .to_string(),
                technical_detail,
            }
        }
        UpdateFailure::StartUpdate(
            StartUpdateError::HeaderCommonParamError | StartUpdateError::HeaderOtherError,
        )
        | UpdateFailure::WriteUpdateImage(
            WriteUpdateImageError::WriteImageCommonParamError
            | WriteUpdateImageError::WriteImageOtherError,
        )
        | UpdateFailure::VerifyUpdateImage(
            VerifyUpdateImageError::VerifyCommonParamError
            | VerifyUpdateImageError::VerifyOtherError,
        )
        | UpdateFailure::FinalizeUpdate(FinalizeUpdateError::FinalizeOtherError) => FriendlyError {
            error_title: "Controller rejected the firmware update".to_string(),
            error_message:
                "The controller reported that it could not apply this firmware update.".to_string(),
            recovery_hint:
                "Confirm you chose the matching controller model, keep the USB cable firmly connected, and scan the controller again to check its current version."
                    .to_string(),
            technical_detail,
        },
    }
}

fn sanitize_network_message(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.contains("ureq::")
        || trimmed.contains("panicked")
        || trimmed.contains("stack backtrace")
    {
        return "Could not download firmware data from Sony's update server.".to_string();
    }
    trimmed.to_string()
}
