// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT AND GPL-3.0-or-later

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Duration;

use crate::error::{CoreError, Result};
use crate::protocol::{
    DEFAULT_PID, DEFAULT_VID, DUALSENSE_EDGE_PID, MAX_HID_CHUNK_SIZE, REPORT_ID_SERIAL_COMMAND,
    REPORT_ID_SERIAL_RESPONSE, REPORT_ID_UPDATE_COMMAND, REPORT_ID_UPDATE_STATUS, UpdateCommand,
};
use crate::transport::*;

#[derive(Default)]
struct MockTransport {
    sent: RefCell<Vec<Vec<u8>>>,
    responses: RefCell<VecDeque<Result<Vec<u8>>>>,
    fail_send: bool,
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
        if self.fail_send {
            return Err(CoreError::Hid("send failed".into()));
        }
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

#[test]
fn try_read_serial_number_parses_valid_0x81_response_and_handles_failures() {
    // Valid 64-byte 0x81 response
    let mock = MockTransport::default();
    let mut resp = vec![0u8; 64];
    resp[0] = REPORT_ID_SERIAL_RESPONSE;
    resp[1] = 1;
    resp[2] = 19;
    resp[3] = 2;
    resp[4..21].copy_from_slice(b"F12301ABCDEF12345");
    mock.push_response(Ok(resp));

    let dev = DualSenseDevice::new(mock);
    assert_eq!(
        dev.try_read_serial_number().as_deref(),
        Some("F12301ABCDEF12345")
    );
    assert_eq!(
        dev.transport().sent.borrow().as_slice(),
        &[vec![REPORT_ID_SERIAL_COMMAND, 1, 19]]
    );

    // Send failure returns None cleanly
    let fail_send = MockTransport {
        fail_send: true,
        ..MockTransport::default()
    };
    assert_eq!(
        DualSenseDevice::new(fail_send).try_read_serial_number(),
        None
    );

    // Get failure returns None cleanly
    let fail_get = MockTransport::default();
    fail_get.push_response(Err(CoreError::Hid("unsupported".into())));
    assert_eq!(
        DualSenseDevice::new(fail_get).try_read_serial_number(),
        None
    );

    // Too short (< 21) returns None
    let short = MockTransport::default();
    short.push_response(Ok(vec![0x81, 1, 19, 2, b'A', b'B']));
    assert_eq!(DualSenseDevice::new(short).try_read_serial_number(), None);

    // Invalid header bytes return None
    for bad_hdr in [
        [0x80, 1, 19, 2],
        [0x81, 2, 19, 2],
        [0x81, 1, 18, 2],
        [0x81, 1, 19, 3],
    ] {
        let mock_bad = MockTransport::default();
        let mut buf = vec![0u8; 64];
        buf[..4].copy_from_slice(&bad_hdr);
        buf[4..21].copy_from_slice(b"F12301ABCDEF12345");
        mock_bad.push_response(Ok(buf));
        assert_eq!(
            DualSenseDevice::new(mock_bad).try_read_serial_number(),
            None
        );
    }

    // Empty / NUL serial returns None
    let empty_serial = MockTransport::default();
    let mut buf_empty = vec![0u8; 64];
    buf_empty[0] = 0x81;
    buf_empty[1] = 1;
    buf_empty[2] = 19;
    buf_empty[3] = 2;
    empty_serial.push_response(Ok(buf_empty));
    assert_eq!(
        DualSenseDevice::new(empty_serial).try_read_serial_number(),
        None
    );
}
