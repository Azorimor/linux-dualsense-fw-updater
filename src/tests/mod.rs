// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

mod licenses_test;
mod quick_style_test;
mod snapshot_adapter_test;
mod snapshot_state_test;

use dualsense_core::{
    DEFAULT_PID, DEFAULT_VID, DUALSENSE_EDGE_PID, FirmwareCatalog, FirmwareInfo, FirmwareRelease,
    HidDeviceInfo, REPORT_ID_FIRMWARE_INFO, decode_ds5_color, resolve_controller_target,
};
use dualsense_ui::DeviceInspection;

pub(super) fn sample_hid_device(path: &str, pid: u16, serial: &str) -> HidDeviceInfo {
    HidDeviceInfo {
        path: path.to_string(),
        vendor_id: DEFAULT_VID,
        product_id: pid,
        interface_number: 3,
        usage_page: 0x0001,
        usage: 0x0005,
        product_string: if pid == DUALSENSE_EDGE_PID {
            "DualSense Edge Wireless Controller".to_string()
        } else {
            "DualSense Wireless Controller".to_string()
        },
        serial_number: serial.to_string(),
    }
}

pub(super) fn sample_firmware_info(
    build_date: &str,
    build_time: &str,
    version: u16,
    fw_series: u16,
    hw_info: u32,
) -> FirmwareInfo {
    let mut raw = vec![0u8; 64];
    raw[0] = REPORT_ID_FIRMWARE_INFO;
    let date_bytes = build_date.as_bytes();
    let date_len = date_bytes.len().min(11);
    raw[1..1 + date_len].copy_from_slice(&date_bytes[..date_len]);
    let time_bytes = build_time.as_bytes();
    let time_len = time_bytes.len().min(8);
    raw[12..12 + time_len].copy_from_slice(&time_bytes[..time_len]);
    raw[22..24].copy_from_slice(&fw_series.to_le_bytes());
    raw[24..28].copy_from_slice(&hw_info.to_le_bytes());
    raw[44..46].copy_from_slice(&version.to_le_bytes());
    FirmwareInfo::from_report(raw).expect("valid firmware info")
}

pub(super) fn sample_inspection(
    build_date: &str,
    build_time: &str,
    version: u16,
    pid: u16,
    fw_series: u16,
    hw_info: u32,
    serial: Option<&str>,
) -> DeviceInspection {
    let firmware_info = sample_firmware_info(build_date, build_time, version, fw_series, hw_info);
    let hardware = resolve_controller_target(&firmware_info, pid);
    let serial_number = serial.map(ToString::to_string);
    let color_name = serial.and_then(decode_ds5_color).map(ToString::to_string);
    DeviceInspection {
        firmware_info,
        hardware,
        serial_number,
        color_name,
    }
}

pub(super) fn sample_bdm020_inspection(version: u16) -> DeviceInspection {
    sample_inspection(
        "Jan 01 2024",
        "00:00:00",
        version,
        DEFAULT_PID,
        0x0004,
        0x0000_0411,
        None,
    )
}

pub(super) fn sample_catalog_for_0004(version: u16) -> FirmwareCatalog {
    FirmwareCatalog {
        application_latest_version: Some("2.2.1.2".to_string()),
        releases: vec![FirmwareRelease {
            target_id: "0004".to_string(),
            version,
            version_raw: format!("0x{version:04x}"),
            download_url: format!(
                "https://fwupdater.dl.playstation.net/fwupdater/fwupdate0004/0x{version:04x}/FWUPDATE0004.bin"
            ),
            target_info: None,
        }],
    }
}
