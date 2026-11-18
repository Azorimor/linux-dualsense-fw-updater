// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT AND GPL-3.0-or-later

use std::ffi::{CStr, CString};
use std::time::Duration;

use hidapi::{HidApi, HidDevice};

use crate::error::{CoreError, Result};
use crate::protocol::{
    BatteryStatus, DEFAULT_PID, DEFAULT_VID, DUALSENSE_EDGE_PID, FIRMWARE_INFO_REPORT_LEN,
    FirmwareInfo, INPUT_REPORT_USB_LEN, REPORT_ID_FIRMWARE_INFO, REPORT_ID_SERIAL_COMMAND,
    REPORT_ID_SERIAL_RESPONSE, REPORT_ID_UPDATE_STATUS, SERIAL_REPORT_LEN, UpdateCommand,
    UpdateStatus, decode_ascii, encode_update_command_chunks,
};

/// Default timeout (in milliseconds) for best-effort input report reads during device inspection.
pub const BATTERY_READ_TIMEOUT_MS: i32 = 50;

/// Underlying HID transport bus type for an enumerated DualSense device.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Hash)]
pub enum HidBusType {
    /// Connected over USB (supports firmware inspection and updates).
    Usb,
    /// Connected over Bluetooth (firmware updates require a USB cable).
    Bluetooth,
    /// Bus type could not be determined by the HID backend.
    #[default]
    Unknown,
}

impl HidBusType {
    /// Maps a [`hidapi::BusType`] with an `interface_number >= 0` fallback into [`HidBusType`].
    pub fn from_hidapi(bus: hidapi::BusType, interface_number: i32) -> Self {
        match bus {
            hidapi::BusType::Usb => Self::Usb,
            hidapi::BusType::Bluetooth => Self::Bluetooth,
            _ if interface_number >= 0 => Self::Usb,
            _ => Self::Unknown,
        }
    }

    /// Returns a human-readable label (`"USB"`, `"Bluetooth"`, or `"Unknown"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Usb => "USB",
            Self::Bluetooth => "Bluetooth",
            Self::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for HidBusType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Metadata describing an enumerated HID DualSense candidate device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidDeviceInfo {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
    pub product_string: String,
    pub serial_number: String,
    pub bus_type: HidBusType,
}

impl HidDeviceInfo {
    /// Returns `true` if this device is connected over Bluetooth.
    pub fn is_bluetooth(&self) -> bool {
        self.bus_type == HidBusType::Bluetooth
    }

    /// Returns `true` if this device is connected over USB.
    pub fn is_usb(&self) -> bool {
        self.bus_type == HidBusType::Usb
    }

    /// Returns a concise human-readable label for device selection lists.
    pub fn display_label(&self) -> String {
        let name = if self.product_string.is_empty() {
            "DualSense Controller"
        } else {
            self.product_string.as_str()
        };
        if self.serial_number.is_empty() {
            format!("{name} ({:04x}:{:04x})", self.vendor_id, self.product_id)
        } else {
            format!("{name} — {}", self.serial_number)
        }
    }
}

/// Returns only devices that are not connected over Bluetooth (USB or unknown bus).
pub fn filter_usb_devices(devices: &[HidDeviceInfo]) -> Vec<HidDeviceInfo> {
    devices
        .iter()
        .filter(|d| !d.is_bluetooth())
        .cloned()
        .collect()
}

/// Returns only devices connected over Bluetooth.
pub fn filter_bluetooth_devices(devices: &[HidDeviceInfo]) -> Vec<HidDeviceInfo> {
    devices
        .iter()
        .filter(|d| d.is_bluetooth())
        .cloned()
        .collect()
}

/// Returns `true` when at least one device is present and every enumerated device is Bluetooth.
pub fn has_only_bluetooth_devices(devices: &[HidDeviceInfo]) -> bool {
    !devices.is_empty() && devices.iter().all(HidDeviceInfo::is_bluetooth)
}

/// Low-level USB HID feature-report transport abstraction for deterministic testing.
pub trait HidTransport {
    /// Reads a HID feature report with leading `report_id` and up to `length` bytes.
    fn get_feature_report(&self, report_id: u8, length: usize) -> Result<Vec<u8>>;

    /// Sends a raw HID feature report payload (including leading report ID).
    fn send_feature_report(&self, data: &[u8]) -> Result<()>;

    /// Reads an HID input report with up to `timeout_ms` milliseconds wait.
    ///
    /// Returns `Ok(None)` when no input report arrives before the timeout expires or when the
    /// transport implementation does not provide input reports.
    fn read_input_report(&self, _timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
}

impl<T: HidTransport + ?Sized> HidTransport for Box<T> {
    fn get_feature_report(&self, report_id: u8, length: usize) -> Result<Vec<u8>> {
        (**self).get_feature_report(report_id, length)
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        (**self).send_feature_report(data)
    }

    fn read_input_report(&self, timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        (**self).read_input_report(timeout_ms)
    }
}

impl<T: HidTransport + ?Sized> HidTransport for &T {
    fn get_feature_report(&self, report_id: u8, length: usize) -> Result<Vec<u8>> {
        (**self).get_feature_report(report_id, length)
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        (**self).send_feature_report(data)
    }

    fn read_input_report(&self, timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        (**self).read_input_report(timeout_ms)
    }
}

/// Device discovery and connection factory abstraction over USB HID.
pub trait HidDeviceEnumerator {
    /// Lists matching HID devices for the given USB vendor and product IDs.
    fn list_devices(&self, vid: u16, pid: u16) -> Result<Vec<HidDeviceInfo>>;

    /// Opens either the specified HID `path` or the first matching `(vid, pid)` device.
    fn open(&self, vid: u16, pid: u16, path: Option<&str>) -> Result<Box<dyn HidTransport>>;

    /// Lists both standard DualSense (`0x054c:0x0ce6`) and DualSense Edge (`0x054c:0x0df2`) devices.
    fn list_dualsense_devices(&self) -> Result<Vec<HidDeviceInfo>> {
        let mut devices = self.list_devices(DEFAULT_VID, DEFAULT_PID)?;
        devices.extend(self.list_devices(DEFAULT_VID, DUALSENSE_EDGE_PID)?);
        Ok(devices)
    }

    /// Lists non-Bluetooth DualSense devices suitable for USB firmware inspection and updates.
    fn list_usb_dualsense_devices(&self) -> Result<Vec<HidDeviceInfo>> {
        Ok(filter_usb_devices(&self.list_dualsense_devices()?))
    }
}

/// Timing abstraction for polling delays so update loops can be tested without real sleeps.
pub trait Sleeper {
    /// Pauses execution for `duration`.
    fn sleep(&self, duration: Duration);
}

/// Standard library [`Sleeper`] backed by [`std::thread::sleep`].
#[derive(Debug, Default, Copy, Clone)]
pub struct StdSleeper;

impl Sleeper for StdSleeper {
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// Protocol-level DualSense HID client built atop any [`HidTransport`].
#[derive(Debug)]
pub struct DualSenseDevice<T> {
    transport: T,
}

impl<T: HidTransport> DualSenseDevice<T> {
    /// Wraps a [`HidTransport`] implementation.
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    /// Returns a reference to the underlying transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Unwraps the underlying transport.
    pub fn into_inner(self) -> T {
        self.transport
    }

    /// Reads and parses Feature Report `0x20` (`FirmwareInfo`).
    pub fn get_firmware_info(&self) -> Result<FirmwareInfo> {
        let raw = self
            .transport
            .get_feature_report(REPORT_ID_FIRMWARE_INFO, FIRMWARE_INFO_REPORT_LEN)?;
        FirmwareInfo::from_report(raw)
    }

    /// Best-effort read of the controller serial number via Feature Reports `0x80`/`0x81`.
    ///
    /// Returns `None` without erroring if the transport rejects `0x80`/`0x81` or returns invalid data.
    pub fn try_read_serial_number(&self) -> Option<String> {
        if self
            .transport
            .send_feature_report(&[REPORT_ID_SERIAL_COMMAND, 1, 19])
            .is_err()
        {
            return None;
        }
        let raw = self
            .transport
            .get_feature_report(REPORT_ID_SERIAL_RESPONSE, SERIAL_REPORT_LEN)
            .ok()?;
        if raw.len() < 21
            || raw[0] != REPORT_ID_SERIAL_RESPONSE
            || raw[1] != 1
            || raw[2] != 19
            || raw[3] != 2
        {
            return None;
        }
        let serial = decode_ascii(&raw[4..21]).trim().to_string();
        if serial.is_empty() {
            None
        } else {
            Some(serial)
        }
    }

    /// Best-effort read of the controller battery status via USB HID input report `0x01`.
    ///
    /// Never fails device inspection; returns `None` if no valid `0x01` report arrives within a
    /// short timeout or if the transport reports an error.
    pub fn try_read_battery_status(&self) -> Option<BatteryStatus> {
        for _ in 0..3 {
            let Ok(Some(raw)) = self.transport.read_input_report(BATTERY_READ_TIMEOUT_MS) else {
                return None;
            };
            if let Some(status) = BatteryStatus::from_input_report(&raw) {
                return Some(status);
            }
        }
        None
    }

    /// Sends an [`UpdateCommand`] and its payload chunked into `<= 0x39`-byte `0xF4` reports.
    pub fn send_update_command(&self, command: UpdateCommand, payload: &[u8]) -> Result<()> {
        let packets = encode_update_command_chunks(command, payload);
        let mut offset = 0usize;
        for packet in packets {
            let chunk = &packet[3..];
            let preview = chunk
                .iter()
                .take(4)
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            log::debug!(
                "F4 chunk off={} len={} first4={}",
                offset,
                chunk.len(),
                preview
            );
            self.transport.send_feature_report(&packet)?;
            offset += chunk.len();
        }
        Ok(())
    }

    /// Reads and validates Feature Report `0xF5` (`UpdateStatus`).
    pub fn get_update_status(&self, length: usize) -> Result<UpdateStatus> {
        let raw = self
            .transport
            .get_feature_report(REPORT_ID_UPDATE_STATUS, length)?;
        let dump = raw
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        log::debug!("F5 status raw: {dump}");
        UpdateStatus::from_report(raw)
    }
}

/// Live `hidapi` feature-report transport wrapping an open `HidDevice`.
pub struct HidApiTransport {
    _api: HidApi,
    dev: HidDevice,
}

impl HidTransport for HidApiTransport {
    fn get_feature_report(&self, report_id: u8, length: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; length];
        if !buf.is_empty() {
            buf[0] = report_id;
        }
        let size = self.dev.get_feature_report(&mut buf)?;
        buf.truncate(size);
        Ok(buf)
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        self.dev.send_feature_report(data)?;
        Ok(())
    }

    fn read_input_report(&self, timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        let mut buf = vec![0u8; INPUT_REPORT_USB_LEN];
        let size = self.dev.read_timeout(&mut buf, timeout_ms)?;
        if size == 0 {
            return Ok(None);
        }
        buf.truncate(size);
        Ok(Some(buf))
    }
}

/// Live `hidapi` backend for enumerating and opening DualSense USB HID controllers.
#[derive(Debug, Default, Copy, Clone)]
pub struct HidApiBackend;

impl HidApiBackend {
    /// Opens a live [`HidApiTransport`] for the given `(vid, pid)` and optional `path`.
    pub fn open_transport(vid: u16, pid: u16, path: Option<&str>) -> Result<HidApiTransport> {
        let api = HidApi::new()?;
        let selected_path = path.filter(|p| !p.is_empty());
        let dev = if let Some(path_str) = selected_path {
            if let Ok(c_path) = CString::new(path_str) {
                api.open_path(&c_path)?
            } else {
                let device_path = find_path(&api, vid, pid, path_str)?;
                api.open_path(device_path)?
            }
        } else {
            let mut iter = api
                .device_list()
                .filter(|d| d.vendor_id() == vid && d.product_id() == pid);
            let device = iter.next().ok_or(CoreError::DeviceNotFound { vid, pid })?;
            device.open_device(&api)?
        };
        Ok(HidApiTransport { _api: api, dev })
    }
}

impl HidDeviceEnumerator for HidApiBackend {
    fn list_devices(&self, vid: u16, pid: u16) -> Result<Vec<HidDeviceInfo>> {
        let api = HidApi::new()?;
        let devices = api
            .device_list()
            .filter(|d| d.vendor_id() == vid && d.product_id() == pid)
            .map(|d| {
                let interface_number = d.interface_number();
                HidDeviceInfo {
                    path: d.path().to_string_lossy().to_string(),
                    vendor_id: d.vendor_id(),
                    product_id: d.product_id(),
                    interface_number,
                    usage_page: d.usage_page(),
                    usage: d.usage(),
                    product_string: d.product_string().unwrap_or("").to_string(),
                    serial_number: d.serial_number().unwrap_or("").to_string(),
                    bus_type: HidBusType::from_hidapi(d.bus_type(), interface_number),
                }
            })
            .collect();
        Ok(devices)
    }

    fn open(&self, vid: u16, pid: u16, path: Option<&str>) -> Result<Box<dyn HidTransport>> {
        Ok(Box::new(Self::open_transport(vid, pid, path)?))
    }
}

fn find_path<'a>(api: &'a HidApi, vid: u16, pid: u16, path_str: &str) -> Result<&'a CStr> {
    for device in api
        .device_list()
        .filter(|d| d.vendor_id() == vid && d.product_id() == pid)
    {
        let path = device.path();
        if path.to_string_lossy() == path_str {
            return Ok(path);
        }
    }
    Err(CoreError::DevicePathNotMatched(path_str.to_string()))
}
