mod error_map_test;
mod http_test;
mod viewmodel_board_test;
mod viewmodel_flow_test;

use dualsense_core::{
    DEFAULT_PID, DEFAULT_VID, DUALSENSE_EDGE_PID, FirmwareCatalog, FirmwareInfo, HidDeviceInfo,
    REPORT_ID_FIRMWARE_INFO, decode_ds5_color, resolve_controller_target,
};

use crate::viewmodel::DeviceInspection;

pub(super) fn sample_hid_device(path: &str, pid: u16, serial: &str) -> HidDeviceInfo {
    HidDeviceInfo {
        path: path.to_string(),
        vendor_id: DEFAULT_VID,
        product_id: pid,
        interface_number: 3,
        usage_page: 1,
        usage: 5,
        product_string: if pid == DUALSENSE_EDGE_PID {
            "DualSense Edge Wireless Controller".to_string()
        } else {
            "DualSense Wireless Controller".to_string()
        },
        serial_number: serial.to_string(),
    }
}

pub(super) fn sample_firmware_info_with_hw(
    version: u16,
    fw_series: u16,
    hw_info: u32,
) -> FirmwareInfo {
    let mut raw = vec![0u8; 64];
    raw[0] = REPORT_ID_FIRMWARE_INFO;
    raw[1..12].copy_from_slice(b"Oct  1 2024");
    raw[12..20].copy_from_slice(b"12:34:56");
    raw[22..24].copy_from_slice(&fw_series.to_le_bytes());
    raw[24..28].copy_from_slice(&hw_info.to_le_bytes());
    raw[44..46].copy_from_slice(&version.to_le_bytes());
    FirmwareInfo::from_report(raw).expect("valid firmware info")
}

pub(super) fn sample_inspection(
    version: u16,
    pid: u16,
    fw_series: u16,
    hw_info: u32,
    serial: Option<&str>,
) -> DeviceInspection {
    let fw_info = sample_firmware_info_with_hw(version, fw_series, hw_info);
    let hardware = resolve_controller_target(&fw_info, pid);
    let serial_number = serial.map(ToString::to_string);
    let color_name = serial.and_then(decode_ds5_color).map(ToString::to_string);
    DeviceInspection {
        firmware_info: fw_info,
        hardware,
        serial_number,
        color_name,
    }
}

pub(super) fn sample_bdm020_inspection(version: u16) -> DeviceInspection {
    sample_inspection(version, DEFAULT_PID, 0x0004, 0x0000_0411, None)
}

pub(super) fn sample_edge_inspection(version: u16) -> DeviceInspection {
    sample_inspection(version, DUALSENSE_EDGE_PID, 0x0044, 0x0000_0400, None)
}

pub(super) fn sample_catalog() -> FirmwareCatalog {
    FirmwareCatalog::from_json(
        r#"{"ApplicationLatestVersion":"2.2.1.2","FwUpdate0004LatestVersion":"0x0630","FwUpdate000BLatestVersion":"0x0630","FwUpdate000ELatestVersion":"0x0641","FwUpdate0044LatestVersion":"0x0217"}"#,
    )
    .expect("valid catalog")
}
