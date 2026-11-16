//! Core library for DualSense USB HID firmware inspection and updates.

pub mod cancel;
pub mod catalog;
pub mod error;
pub mod image;
pub mod protocol;
pub mod transport;
pub mod updater;

pub use cancel::CancellationToken;
pub use catalog::{
    DownloadProgress, FirmwareCatalog, FirmwareCatalogClient, FirmwareRelease, FirmwareTargetInfo,
    HttpClient, KNOWN_TARGETS, SONY_FW_BASE_URL, SONY_INFO_JSON_URL, build_firmware_url,
    find_known_target,
};
pub use error::{
    CoreError, FinalizeUpdateError, Result, StartUpdateError, UpdateFailure, UpdatePhase,
    VerifyUpdateImageError, WriteUpdateImageError,
};
pub use image::FirmwareImage;
pub use protocol::{
    DEFAULT_PID, DEFAULT_VID, DUALSENSE_EDGE_PID, FIRMWARE_INFO_REPORT_LEN,
    FIRMWARE_VERSION_OFFSET, FirmwareInfo, MAX_HID_CHUNK_SIZE, REPORT_ID_FIRMWARE_INFO,
    REPORT_ID_UPDATE_COMMAND, REPORT_ID_UPDATE_STATUS, StartUpdateStatusCode, UPDATE_HEADER_LEN,
    UPDATE_STATUS_REPORT_LEN, UpdateCommand, UpdateStatus, VerifyUpdateStatusCode,
    WRITE_BLOCK_SIZE, WriteUpdateStatusCode, decode_ascii, encode_update_command_chunks,
    encode_update_report,
};
pub use transport::{
    DualSenseDevice, HidApiBackend, HidApiTransport, HidDeviceEnumerator, HidDeviceInfo,
    HidTransport, Sleeper, StdSleeper,
};
pub use updater::{DualSenseUpdater, UpdateConfig, UpdateProgress};
