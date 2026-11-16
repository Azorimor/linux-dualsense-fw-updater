use std::ffi::{CStr, CString};
use std::time::Duration;

use hidapi::{HidApi, HidDevice};

use crate::error::{CoreError, Result};
use crate::protocol::{
    DEFAULT_PID, DEFAULT_VID, DUALSENSE_EDGE_PID, FIRMWARE_INFO_REPORT_LEN, FirmwareInfo,
    REPORT_ID_FIRMWARE_INFO, REPORT_ID_UPDATE_STATUS, UpdateCommand, UpdateStatus,
    encode_update_command_chunks,
};

/// Metadata describing an enumerated USB HID DualSense candidate device.
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
}

impl HidDeviceInfo {
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

/// Low-level USB HID feature-report transport abstraction for deterministic testing.
pub trait HidTransport {
    /// Reads a HID feature report with leading `report_id` and up to `length` bytes.
    fn get_feature_report(&self, report_id: u8, length: usize) -> Result<Vec<u8>>;

    /// Sends a raw HID feature report payload (including leading report ID).
    fn send_feature_report(&self, data: &[u8]) -> Result<()>;
}

impl<T: HidTransport + ?Sized> HidTransport for Box<T> {
    fn get_feature_report(&self, report_id: u8, length: usize) -> Result<Vec<u8>> {
        (**self).get_feature_report(report_id, length)
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        (**self).send_feature_report(data)
    }
}

impl<T: HidTransport + ?Sized> HidTransport for &T {
    fn get_feature_report(&self, report_id: u8, length: usize) -> Result<Vec<u8>> {
        (**self).get_feature_report(report_id, length)
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        (**self).send_feature_report(data)
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
            let device = iter
                .next()
                .ok_or(CoreError::DeviceNotFound { vid, pid })?;
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
            .map(|d| HidDeviceInfo {
                path: d.path().to_string_lossy().to_string(),
                vendor_id: d.vendor_id(),
                product_id: d.product_id(),
                interface_number: d.interface_number(),
                usage_page: d.usage_page(),
                usage: d.usage(),
                product_string: d.product_string().unwrap_or("").to_string(),
                serial_number: d.serial_number().unwrap_or("").to_string(),
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use super::*;
    use crate::protocol::{MAX_HID_CHUNK_SIZE, REPORT_ID_UPDATE_COMMAND};

    #[derive(Default)]
    struct MockTransport {
        sent: RefCell<Vec<Vec<u8>>>,
        responses: RefCell<VecDeque<Result<Vec<u8>>>>,
    }

    impl MockTransport {
        fn push_response(&self, resp: Result<Vec<u8>>) {
            self.responses.borrow_mut().push_back(resp);
        }
    }

    impl HidTransport for MockTransport {
        fn get_feature_report(&self, report_id: u8, length: usize) -> Result<Vec<u8>> {
            let next = self
                .responses
                .borrow_mut()
                .pop_front()
                .expect("unexpected get_feature_report");
            next.map(|mut buf| {
                if !buf.is_empty() && buf[0] == 0 {
                    let _ = report_id;
                }
                buf.truncate(length.max(buf.len()));
                buf
            })
        }

        fn send_feature_report(&self, data: &[u8]) -> Result<()> {
            self.sent.borrow_mut().push(data.to_vec());
            Ok(())
        }
    }

    #[test]
    fn device_info_display_label_handles_missing_and_present_metadata() {
        let plain = HidDeviceInfo {
            path: "/dev/hidraw0".into(),
            vendor_id: DEFAULT_VID,
            product_id: DEFAULT_PID,
            interface_number: 3,
            usage_page: 0x0001,
            usage: 0x0005,
            product_string: String::new(),
            serial_number: String::new(),
        };
        assert_eq!(plain.display_label(), "DualSense Controller (054c:0ce6)");

        let named = HidDeviceInfo {
            product_string: "DualSense Wireless Controller".into(),
            serial_number: "AA:BB:CC".into(),
            ..plain
        };
        assert_eq!(
            named.display_label(),
            "DualSense Wireless Controller — AA:BB:CC"
        );
    }

    #[test]
    fn dualsense_device_reads_firmware_info_and_status() {
        let mock = MockTransport::default();
        let mut fw_report = vec![0u8; 64];
        fw_report[..11].copy_from_slice(b"Mar 10 2025");
        fw_report[12..20].copy_from_slice(b"09:15:00");
        fw_report[44] = 0x30;
        fw_report[45] = 0x06;
        mock.push_response(Ok(fw_report));
        mock.push_response(Ok(vec![REPORT_ID_UPDATE_STATUS, 0x00, 0x04, 0x00]));

        let dev = DualSenseDevice::new(mock);
        let info = dev.get_firmware_info().expect("firmware info");
        assert_eq!(info.firmware_version, 0x0630);
        assert_eq!(info.build_date, "Mar 10 2025");

        let status = dev.get_update_status(4).expect("update status");
        assert_eq!(status.command, UpdateCommand::StartUpdate);
        assert_eq!(status.status_raw, 0x04);
    }

    #[test]
    fn dualsense_device_chunks_update_commands_and_handles_boxed_transport() {
        let mock = MockTransport::default();
        let boxed: Box<dyn HidTransport> = Box::new(mock);
        let dev = DualSenseDevice::new(boxed);
        let payload = vec![0x5Au8; MAX_HID_CHUNK_SIZE + 5];
        dev.send_update_command(UpdateCommand::StartUpdate, &payload)
            .expect("send command");
        dev.send_update_command(UpdateCommand::FinalizeUpdate, &[])
            .expect("send empty command");

        // Verify StdSleeper with zero duration works cleanly in tests.
        StdSleeper.sleep(Duration::ZERO);
        let _ = dev.into_inner();

        // Direct test on unboxed MockTransport to assert exact chunk headers.
        let mock2 = MockTransport::default();
        let dev2 = DualSenseDevice::new(&mock2);
        dev2.send_update_command(UpdateCommand::StartUpdate, &payload)
            .expect("send command");
        let sent = mock2.sent.borrow();
        assert_eq!(sent.len(), 2);
        assert_eq!(
            sent[0][..3],
            [REPORT_ID_UPDATE_COMMAND, 0x00, MAX_HID_CHUNK_SIZE as u8]
        );
        assert_eq!(sent[1][..3], [REPORT_ID_UPDATE_COMMAND, 0x00, 5]);
        assert_eq!(dev2.transport().sent.borrow().len(), 2);
    }

    struct StubEnumerator;
    impl HidDeviceEnumerator for StubEnumerator {
        fn list_devices(&self, vid: u16, pid: u16) -> Result<Vec<HidDeviceInfo>> {
            Ok(vec![HidDeviceInfo {
                path: format!("/dev/{vid:04x}:{pid:04x}"),
                vendor_id: vid,
                product_id: pid,
                interface_number: 3,
                usage_page: 1,
                usage: 5,
                product_string: "Stub".into(),
                serial_number: "123".into(),
            }])
        }

        fn open(&self, _vid: u16, _pid: u16, _path: Option<&str>) -> Result<Box<dyn HidTransport>> {
            Ok(Box::new(MockTransport::default()))
        }
    }

    #[test]
    fn list_dualsense_devices_queries_standard_and_edge_pids() {
        let enumerator = StubEnumerator;
        let all = enumerator.list_dualsense_devices().expect("list");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].product_id, DEFAULT_PID);
        assert_eq!(all[1].product_id, DUALSENSE_EDGE_PID);
    }
}
