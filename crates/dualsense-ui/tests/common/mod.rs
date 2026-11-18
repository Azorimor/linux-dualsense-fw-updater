#![allow(dead_code)]

// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use dualsense_core::{
    BatteryChargingState, CancellationToken, CoreError, DUALSENSE_EDGE_PID, DownloadProgress,
    FIRMWARE_INFO_REPORT_LEN, FIRMWARE_PID_OFFSET, FIRMWARE_TARGET_OFFSET, FIRMWARE_VERSION_OFFSET,
    HidBusType, HidDeviceEnumerator, HidDeviceInfo, HidTransport, HttpClient,
    INPUT_REPORT_BATTERY_OFFSET, INPUT_REPORT_USB_LEN, REPORT_ID_FIRMWARE_INFO, REPORT_ID_INPUT,
    REPORT_ID_SERIAL_RESPONSE, REPORT_ID_UPDATE_STATUS, Result, SERIAL_REPORT_LEN, Sleeper,
    StartUpdateStatusCode, UpdateCommand, UpdateConfig, VerifyUpdateStatusCode, WRITE_BLOCK_SIZE,
    WriteUpdateStatusCode,
};
use dualsense_ui::UiSnapshot;
use tokio::sync::watch;

pub const SAMPLE_CATALOG_JSON: &str = r#"{"ApplicationLatestVersion":"2.2.1.2","FwUpdate0004LatestVersion":"0x0630","FwUpdate000BLatestVersion":"0x0630","FwUpdate000ELatestVersion":"0x0641","FwUpdate0044LatestVersion":"0x0217"}"#;

#[derive(Debug, Clone, Default)]
pub struct NoopSleeper;

impl Sleeper for NoopSleeper {
    fn sleep(&self, _duration: Duration) {}
}

pub struct SharedMockTransport {
    sent_reports: Arc<Mutex<Vec<Vec<u8>>>>,
    responses: Arc<Mutex<VecDeque<Result<Vec<u8>>>>>,
    serial_response: Arc<Mutex<Option<Result<Vec<u8>>>>>,
    battery_report: Arc<Mutex<Option<Result<Vec<u8>>>>>,
    response_delay: Option<Duration>,
}

impl HidTransport for SharedMockTransport {
    fn get_feature_report(&self, report_id: u8, _length: usize) -> Result<Vec<u8>> {
        if let Some(delay) = self.response_delay {
            thread::sleep(delay);
        }
        if report_id == REPORT_ID_SERIAL_RESPONSE {
            return self
                .serial_response
                .lock()
                .expect("lock serial")
                .clone()
                .unwrap_or_else(|| Err(CoreError::Hid("0x81 unsupported in mock".to_string())));
        }
        self.responses
            .lock()
            .expect("lock responses")
            .pop_front()
            .unwrap_or(Err(CoreError::UpdateStatusEmpty))
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        self.sent_reports
            .lock()
            .expect("lock sent")
            .push(data.to_vec());
        Ok(())
    }

    fn read_input_report(&self, _timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        match self.battery_report.lock().expect("lock battery").clone() {
            Some(Ok(report)) => Ok(Some(report)),
            Some(Err(err)) => Err(err),
            None => Ok(None),
        }
    }
}

type ResponseQueueMap = Mutex<BTreeMap<String, Arc<Mutex<VecDeque<Result<Vec<u8>>>>>>>;
type SerialResponseMap = Mutex<BTreeMap<String, Arc<Mutex<Option<Result<Vec<u8>>>>>>>;
type BatteryReportMap = Mutex<BTreeMap<String, Arc<Mutex<Option<Result<Vec<u8>>>>>>>;
type SentReportsMap = Mutex<BTreeMap<String, Arc<Mutex<Vec<Vec<u8>>>>>>;

pub struct MockEnumerator {
    list_result: Mutex<Result<Vec<HidDeviceInfo>>>,
    queues: ResponseQueueMap,
    serial_by_path: SerialResponseMap,
    battery_by_path: BatteryReportMap,
    sent_by_path: SentReportsMap,
    open_error: Mutex<Option<CoreError>>,
    list_calls: AtomicUsize,
    report_delay: Mutex<Option<Duration>>,
}

impl Default for MockEnumerator {
    fn default() -> Self {
        Self {
            list_result: Mutex::new(Ok(Vec::new())),
            queues: Mutex::new(BTreeMap::new()),
            serial_by_path: Mutex::new(BTreeMap::new()),
            battery_by_path: Mutex::new(BTreeMap::new()),
            sent_by_path: Mutex::new(BTreeMap::new()),
            open_error: Mutex::new(None),
            list_calls: AtomicUsize::new(0),
            report_delay: Mutex::new(None),
        }
    }
}

impl MockEnumerator {
    pub fn with_devices(devices: Vec<HidDeviceInfo>) -> Self {
        Self {
            list_result: Mutex::new(Ok(devices)),
            ..Default::default()
        }
    }

    pub fn set_list_result(&self, result: Result<Vec<HidDeviceInfo>>) {
        *self.list_result.lock().expect("lock list") = result;
    }

    pub fn list_call_count(&self) -> usize {
        self.list_calls.load(Ordering::SeqCst)
    }

    pub fn reset_list_call_count(&self) {
        self.list_calls.store(0, Ordering::SeqCst);
    }

    pub fn set_report_delay(&self, delay: Option<Duration>) {
        *self.report_delay.lock().expect("lock report delay") = delay;
    }

    pub fn push_report(&self, path: &str, report: Result<Vec<u8>>) {
        let queue_arc = self
            .queues
            .lock()
            .expect("lock queues")
            .entry(path.to_string())
            .or_default()
            .clone();
        queue_arc.lock().expect("lock queue").push_back(report);
    }

    pub fn push_firmware_info(&self, path: &str, version: u16) {
        self.push_firmware_info_with_hw(path, version, 0x0004, 0x0000_0411);
    }

    pub fn push_firmware_info_with_hw(
        &self,
        path: &str,
        version: u16,
        fw_series: u16,
        hw_info: u32,
    ) {
        self.push_report(
            path,
            Ok(make_firmware_info_report(version, fw_series, hw_info)),
        );
    }

    pub fn set_serial_number(&self, path: &str, serial: &str) {
        let mut raw = vec![0u8; SERIAL_REPORT_LEN];
        raw[0] = REPORT_ID_SERIAL_RESPONSE;
        raw[1] = 1;
        raw[2] = 19;
        raw[3] = 2;
        let bytes = serial.as_bytes();
        let copy_len = bytes.len().min(17);
        raw[4..4 + copy_len].copy_from_slice(&bytes[..copy_len]);
        let slot = self
            .serial_by_path
            .lock()
            .expect("lock serial map")
            .entry(path.to_string())
            .or_default()
            .clone();
        *slot.lock().expect("lock serial") = Some(Ok(raw));
    }

    pub fn set_battery_report(&self, path: &str, level_nibble: u8, charging: BatteryChargingState) {
        let high_nibble = match charging {
            BatteryChargingState::Discharging => 0x0,
            BatteryChargingState::Charging => 0x1,
            BatteryChargingState::Full => 0x2,
            BatteryChargingState::Fault => 0xB,
            BatteryChargingState::Unknown(v) => v & 0x0F,
        };
        let mut raw = vec![0u8; INPUT_REPORT_USB_LEN];
        raw[0] = REPORT_ID_INPUT;
        raw[INPUT_REPORT_BATTERY_OFFSET] = (high_nibble << 4) | (level_nibble & 0x0F);
        let slot = self
            .battery_by_path
            .lock()
            .expect("lock battery map")
            .entry(path.to_string())
            .or_default()
            .clone();
        *slot.lock().expect("lock battery") = Some(Ok(raw));
    }

    pub fn push_status(&self, path: &str, cmd: UpdateCommand, status_raw: u8) {
        self.push_report(
            path,
            Ok(vec![REPORT_ID_UPDATE_STATUS, cmd as u8, status_raw, 0x00]),
        );
    }

    pub fn push_full_update_success(&self, path: &str, image_len: usize) {
        self.push_status(
            path,
            UpdateCommand::StartUpdate,
            StartUpdateStatusCode::Processing as u8,
        );
        self.push_status(
            path,
            UpdateCommand::StartUpdate,
            StartUpdateStatusCode::Success as u8,
        );

        let block_count = image_len.div_ceil(WRITE_BLOCK_SIZE);
        for block_idx in 0..block_count {
            let remaining = image_len - block_idx * WRITE_BLOCK_SIZE;
            let block_len = remaining.min(WRITE_BLOCK_SIZE);
            let chunk_count = block_len.div_ceil(dualsense_core::MAX_HID_CHUNK_SIZE);
            for chunk_idx in 0..chunk_count {
                let is_last_chunk = chunk_idx + 1 == chunk_count;
                if chunk_idx == 0 {
                    self.push_status(
                        path,
                        UpdateCommand::WriteUpdateImage,
                        WriteUpdateStatusCode::Retry as u8,
                    );
                }
                let code = if is_last_chunk {
                    WriteUpdateStatusCode::Success
                } else {
                    WriteUpdateStatusCode::SendNext
                };
                self.push_status(path, UpdateCommand::WriteUpdateImage, code as u8);
            }
        }

        self.push_status(
            path,
            UpdateCommand::VerifyUpdateImage,
            VerifyUpdateStatusCode::KeepPolling as u8,
        );
        self.push_status(
            path,
            UpdateCommand::VerifyUpdateImage,
            VerifyUpdateStatusCode::Success as u8,
        );
    }

    pub fn sent_reports(&self, path: &str) -> Vec<Vec<u8>> {
        self.sent_by_path
            .lock()
            .expect("lock sent map")
            .get(path)
            .map(|arc| arc.lock().expect("lock reports").clone())
            .unwrap_or_default()
    }
}

impl HidDeviceEnumerator for MockEnumerator {
    fn list_devices(&self, _vid: u16, pid: u16) -> Result<Vec<HidDeviceInfo>> {
        if pid == dualsense_core::DEFAULT_PID {
            self.list_calls.fetch_add(1, Ordering::SeqCst);
        }
        let devices = self.list_result.lock().expect("lock list").clone()?;
        Ok(devices
            .into_iter()
            .filter(|d| d.product_id == pid)
            .collect())
    }

    fn open(&self, _vid: u16, _pid: u16, path: Option<&str>) -> Result<Box<dyn HidTransport>> {
        if let Some(err) = self.open_error.lock().expect("lock open_error").clone() {
            return Err(err);
        }
        let target_path = path.unwrap_or("/dev/hidraw0").to_string();
        let queue = self
            .queues
            .lock()
            .expect("lock queues")
            .entry(target_path.clone())
            .or_default()
            .clone();
        let serial_arc = self
            .serial_by_path
            .lock()
            .expect("lock serial map")
            .entry(target_path.clone())
            .or_default()
            .clone();
        let sent_arc = self
            .sent_by_path
            .lock()
            .expect("lock sent map")
            .entry(target_path.clone())
            .or_default()
            .clone();
        let battery_arc = self
            .battery_by_path
            .lock()
            .expect("lock battery map")
            .entry(target_path)
            .or_default()
            .clone();
        let response_delay = *self.report_delay.lock().expect("lock report delay");
        Ok(Box::new(SharedMockTransport {
            sent_reports: sent_arc,
            responses: queue,
            serial_response: serial_arc,
            battery_report: battery_arc,
            response_delay,
        }))
    }
}

pub struct ArcEnumerator(pub Arc<MockEnumerator>);

impl HidDeviceEnumerator for ArcEnumerator {
    fn list_devices(&self, vid: u16, pid: u16) -> Result<Vec<HidDeviceInfo>> {
        self.0.list_devices(vid, pid)
    }

    fn open(&self, vid: u16, pid: u16, path: Option<&str>) -> Result<Box<dyn HidTransport>> {
        self.0.open(vid, pid, path)
    }
}

pub struct MockHttp {
    catalog_result: Mutex<Result<String>>,
    firmware_bytes: Mutex<Result<Vec<u8>>>,
    slow_cancel_mode: bool,
    download_calls: Arc<AtomicUsize>,
}

impl MockHttp {
    pub fn with_catalog_and_image(catalog_json: &str, image: Vec<u8>) -> Self {
        Self {
            catalog_result: Mutex::new(Ok(catalog_json.to_string())),
            firmware_bytes: Mutex::new(Ok(image)),
            slow_cancel_mode: false,
            download_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn with_download_counter(
        catalog_json: &str,
        image: Vec<u8>,
        download_calls: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            catalog_result: Mutex::new(Ok(catalog_json.to_string())),
            firmware_bytes: Mutex::new(Ok(image)),
            slow_cancel_mode: false,
            download_calls,
        }
    }

    pub fn slow_cancellable(catalog_json: &str) -> Self {
        Self {
            catalog_result: Mutex::new(Ok(catalog_json.to_string())),
            firmware_bytes: Mutex::new(Ok(Vec::new())),
            slow_cancel_mode: true,
            download_calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl HttpClient for MockHttp {
    fn get_text(&self, _url: &str, cancel: &CancellationToken) -> Result<String> {
        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        self.catalog_result.lock().expect("lock catalog").clone()
    }

    fn download_with_progress(
        &self,
        _url: &str,
        cancel: &CancellationToken,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<Vec<u8>> {
        self.download_calls.fetch_add(1, Ordering::SeqCst);
        if self.slow_cancel_mode {
            on_progress(DownloadProgress {
                downloaded_bytes: 25,
                total_bytes: Some(100),
            });
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                if cancel.is_cancelled() {
                    return Err(CoreError::Cancelled);
                }
                thread::sleep(Duration::from_millis(5));
            }
            return Err(CoreError::CatalogHttp(
                "timed out waiting for cancel".to_string(),
            ));
        }

        let bytes = self.firmware_bytes.lock().expect("lock fw").clone()?;
        let total = bytes.len();
        let half = total / 2;
        on_progress(DownloadProgress {
            downloaded_bytes: 0,
            total_bytes: Some(total),
        });
        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        on_progress(DownloadProgress {
            downloaded_bytes: half,
            total_bytes: Some(total),
        });
        if cancel.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        on_progress(DownloadProgress {
            downloaded_bytes: total,
            total_bytes: Some(total),
        });
        Ok(bytes)
    }
}

pub fn make_device(path: &str, pid: u16, serial: &str) -> HidDeviceInfo {
    make_device_with_bus(path, pid, serial, HidBusType::Usb)
}

pub fn make_bluetooth_device(path: &str, pid: u16, serial: &str) -> HidDeviceInfo {
    make_device_with_bus(path, pid, serial, HidBusType::Bluetooth)
}

pub fn make_device_with_bus(
    path: &str,
    pid: u16,
    serial: &str,
    bus_type: HidBusType,
) -> HidDeviceInfo {
    HidDeviceInfo {
        path: path.to_string(),
        vendor_id: dualsense_core::DEFAULT_VID,
        product_id: pid,
        interface_number: if bus_type == HidBusType::Bluetooth {
            -1
        } else {
            3
        },
        usage_page: 1,
        usage: 5,
        product_string: if pid == DUALSENSE_EDGE_PID {
            "DualSense Edge Wireless Controller".to_string()
        } else {
            "DualSense Wireless Controller".to_string()
        },
        serial_number: serial.to_string(),
        bus_type,
    }
}

pub fn make_firmware_info_report(version: u16, fw_series: u16, hw_info: u32) -> Vec<u8> {
    let mut payload64 = vec![0u8; FIRMWARE_INFO_REPORT_LEN];
    payload64[0] = REPORT_ID_FIRMWARE_INFO;
    payload64[1..12].copy_from_slice(b"Oct  1 2024");
    payload64[12..20].copy_from_slice(b"10:20:30");
    payload64[22..24].copy_from_slice(&fw_series.to_le_bytes());
    payload64[24..28].copy_from_slice(&hw_info.to_le_bytes());
    payload64[44..46].copy_from_slice(&version.to_le_bytes());
    payload64
}

pub fn make_firmware_image_bytes(len: usize, version: u16) -> Vec<u8> {
    let mut bytes = vec![0x5Au8; len];
    let le = version.to_le_bytes();
    bytes[FIRMWARE_VERSION_OFFSET] = le[0];
    bytes[FIRMWARE_VERSION_OFFSET + 1] = le[1];
    bytes
}

pub fn make_official_firmware_image_bytes(
    len: usize,
    pid: u16,
    target_id: u16,
    version: u16,
) -> Vec<u8> {
    let mut bytes = make_firmware_image_bytes(len, version);
    let sig = b"Sony Interactive Entertainment Inc.";
    bytes[..sig.len()].copy_from_slice(sig);
    bytes[FIRMWARE_PID_OFFSET..FIRMWARE_PID_OFFSET + 2].copy_from_slice(&pid.to_le_bytes());
    bytes[FIRMWARE_TARGET_OFFSET..FIRMWARE_TARGET_OFFSET + 2]
        .copy_from_slice(&target_id.to_le_bytes());
    bytes
}

pub fn fast_update_config() -> UpdateConfig {
    UpdateConfig {
        poll_interval: Duration::ZERO,
        max_start_polls: 10,
        max_write_chunk_retries: 10,
        max_verify_polls: 10,
    }
}

pub fn wait_until<F>(rx: &mut watch::Receiver<UiSnapshot>, predicate: F) -> UiSnapshot
where
    F: Fn(&UiSnapshot) -> bool,
{
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let current = rx.borrow().clone();
        if predicate(&current) {
            return current;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for UI snapshot condition; last state: step={:?}, busy={}, err={:?}, banner={:?}",
            current.step,
            current.is_busy,
            current.error_title,
            current.banner,
        );
        thread::sleep(Duration::from_millis(5));
    }
}
