// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Duration;

use dualsense_core::{
    CancellationToken, CoreError, DownloadProgress, FIRMWARE_PID_OFFSET, FIRMWARE_TARGET_OFFSET,
    FIRMWARE_VERSION_OFFSET, HttpClient, INPUT_REPORT_BATTERY_OFFSET, INPUT_REPORT_USB_LEN,
    MAX_HID_CHUNK_SIZE, REPORT_ID_FIRMWARE_INFO, REPORT_ID_INPUT, REPORT_ID_SERIAL_COMMAND,
    REPORT_ID_SERIAL_RESPONSE, REPORT_ID_UPDATE_COMMAND, REPORT_ID_UPDATE_STATUS,
    SONY_INFO_JSON_URL, Sleeper, StartUpdateStatusCode, UPDATE_HEADER_LEN, UpdateCommand,
    VerifyUpdateStatusCode, WriteUpdateStatusCode,
};

#[derive(Default)]
pub struct NoopSleeper {
    pub sleep_count: RefCell<usize>,
}

impl Sleeper for NoopSleeper {
    fn sleep(&self, _duration: Duration) {
        *self.sleep_count.borrow_mut() += 1;
    }
}

/// Stateful simulated DualSense controller validating wire-level `0xF4`/`0xF5`/`0x20`/`0x80`/`0x81` reports.
pub struct SimDualSenseController {
    pub current_version: u16,
    pub fw_series: u16,
    pub hw_info: u32,
    pub serial_number: Option<String>,
    pub received_header: RefCell<Vec<u8>>,
    pub received_image: RefCell<Vec<u8>>,
    pub finalized: RefCell<bool>,
    pub status_queue: RefCell<VecDeque<(UpdateCommand, u8)>>,
    pub start_processing_polls: usize,
    pub write_retry_every_n_chunks: usize,
    pub verify_keep_polling_count: usize,
    pub written_chunks: RefCell<usize>,
    pub inject_start_error: Option<StartUpdateStatusCode>,
    pub inject_write_error_at_chunk: Option<(usize, WriteUpdateStatusCode)>,
    pub inject_verify_error: Option<VerifyUpdateStatusCode>,
    pub battery_byte: Option<u8>,
    pub post_flash_version: Option<u16>,
    pub post_flash_build_date: [u8; 11],
    pub post_flash_build_time: [u8; 8],
    pub post_flash_transient_errors: RefCell<usize>,
}

impl SimDualSenseController {
    pub fn new(current_version: u16) -> Self {
        Self {
            current_version,
            fw_series: 0x000B,
            hw_info: 0x0000_0811,
            serial_number: None,
            received_header: RefCell::new(Vec::new()),
            received_image: RefCell::new(Vec::new()),
            finalized: RefCell::new(false),
            status_queue: RefCell::new(VecDeque::new()),
            start_processing_polls: 2,
            write_retry_every_n_chunks: 100,
            verify_keep_polling_count: 2,
            written_chunks: RefCell::new(0),
            inject_start_error: None,
            inject_write_error_at_chunk: None,
            inject_verify_error: None,
            battery_byte: None,
            post_flash_version: None,
            post_flash_build_date: *b"May 01 2025",
            post_flash_build_time: *b"14:30:00",
            post_flash_transient_errors: RefCell::new(0),
        }
    }
}

impl dualsense_core::HidTransport for SimDualSenseController {
    fn get_feature_report(&self, report_id: u8, length: usize) -> dualsense_core::Result<Vec<u8>> {
        if report_id == REPORT_ID_FIRMWARE_INFO {
            let is_finalized = *self.finalized.borrow();
            if is_finalized {
                let mut transient = self.post_flash_transient_errors.borrow_mut();
                if *transient > 0 {
                    *transient -= 1;
                    return Err(CoreError::Hid("controller re-enumerating".into()));
                }
            }
            let version = if is_finalized {
                self.post_flash_version.unwrap_or(self.current_version)
            } else {
                self.current_version
            };
            let mut report = vec![0u8; length.max(64)];
            report[0] = REPORT_ID_FIRMWARE_INFO;
            report[1..12].copy_from_slice(&self.post_flash_build_date);
            report[12..20].copy_from_slice(&self.post_flash_build_time);
            report[22..24].copy_from_slice(&self.fw_series.to_le_bytes());
            report[24..28].copy_from_slice(&self.hw_info.to_le_bytes());
            let le = version.to_le_bytes();
            report[44] = le[0];
            report[45] = le[1];
            return Ok(report);
        }
        if report_id == REPORT_ID_SERIAL_RESPONSE {
            let Some(serial) = &self.serial_number else {
                return Err(CoreError::Hid("serial report unsupported".to_string()));
            };
            let mut report = vec![0u8; length.max(64)];
            report[0] = REPORT_ID_SERIAL_RESPONSE;
            report[1] = 1;
            report[2] = 19;
            report[3] = 2;
            let bytes = serial.as_bytes();
            let copy_len = bytes.len().min(17);
            report[4..4 + copy_len].copy_from_slice(&bytes[..copy_len]);
            return Ok(report);
        }
        if report_id == REPORT_ID_UPDATE_STATUS {
            let (cmd, code) = self
                .status_queue
                .borrow_mut()
                .pop_front()
                .expect("status requested with empty queue");
            return Ok(vec![REPORT_ID_UPDATE_STATUS, cmd as u8, code, 0x00]);
        }
        Err(CoreError::Hid(format!(
            "unexpected report_id 0x{report_id:02x}"
        )))
    }

    fn send_feature_report(&self, data: &[u8]) -> dualsense_core::Result<()> {
        if data == [REPORT_ID_SERIAL_COMMAND, 1, 19] {
            if self.serial_number.is_none() {
                return Err(CoreError::Hid("serial command unsupported".to_string()));
            }
            return Ok(());
        }
        assert!(data.len() >= 3);
        assert_eq!(data[0], REPORT_ID_UPDATE_COMMAND);
        let cmd = UpdateCommand::from_int(data[1]);
        let chunk_len = data[2] as usize;
        assert!(chunk_len <= MAX_HID_CHUNK_SIZE);
        assert_eq!(data.len(), 3 + chunk_len);
        let chunk = &data[3..];

        match cmd {
            UpdateCommand::StartUpdate => {
                let mut hdr = self.received_header.borrow_mut();
                hdr.extend_from_slice(chunk);
                if hdr.len() == UPDATE_HEADER_LEN {
                    let mut q = self.status_queue.borrow_mut();
                    for _ in 0..self.start_processing_polls {
                        q.push_back((
                            UpdateCommand::StartUpdate,
                            StartUpdateStatusCode::Processing as u8,
                        ));
                    }
                    let final_code = self
                        .inject_start_error
                        .unwrap_or(StartUpdateStatusCode::Success);
                    q.push_back((UpdateCommand::StartUpdate, final_code as u8));
                }
            }
            UpdateCommand::WriteUpdateImage => {
                self.received_image.borrow_mut().extend_from_slice(chunk);
                let mut count = self.written_chunks.borrow_mut();
                let idx = *count;
                *count += 1;

                let mut q = self.status_queue.borrow_mut();
                if let Some((err_idx, err_code)) = self.inject_write_error_at_chunk
                    && idx == err_idx
                {
                    q.push_back((UpdateCommand::WriteUpdateImage, err_code as u8));
                    return Ok(());
                }

                if self.write_retry_every_n_chunks > 0 && idx % self.write_retry_every_n_chunks == 0
                {
                    q.push_back((
                        UpdateCommand::WriteUpdateImage,
                        WriteUpdateStatusCode::Retry as u8,
                    ));
                }
                q.push_back((
                    UpdateCommand::WriteUpdateImage,
                    WriteUpdateStatusCode::SendNext as u8,
                ));
            }
            UpdateCommand::VerifyUpdateImage => {
                assert_eq!(chunk_len, 0);
                let mut q = self.status_queue.borrow_mut();
                for _ in 0..self.verify_keep_polling_count {
                    q.push_back((
                        UpdateCommand::VerifyUpdateImage,
                        VerifyUpdateStatusCode::KeepPolling as u8,
                    ));
                }
                let final_code = self
                    .inject_verify_error
                    .unwrap_or(VerifyUpdateStatusCode::Success);
                q.push_back((UpdateCommand::VerifyUpdateImage, final_code as u8));
            }
            UpdateCommand::FinalizeUpdate => {
                assert_eq!(chunk_len, 0);
                *self.finalized.borrow_mut() = true;
            }
            UpdateCommand::Unknown => panic!("unexpected Unknown command"),
        }
        Ok(())
    }

    fn read_input_report(&self, _timeout_ms: i32) -> dualsense_core::Result<Option<Vec<u8>>> {
        let Some(byte) = self.battery_byte else {
            return Ok(None);
        };
        let mut report = vec![0u8; INPUT_REPORT_USB_LEN];
        report[0] = REPORT_ID_INPUT;
        report[INPUT_REPORT_BATTERY_OFFSET] = byte;
        Ok(Some(report))
    }
}

pub struct MockSonyHttp {
    pub image_bytes: Vec<u8>,
}

impl HttpClient for MockSonyHttp {
    fn get_text(&self, url: &str, _cancel: &CancellationToken) -> dualsense_core::Result<String> {
        assert_eq!(url, SONY_INFO_JSON_URL);
        Ok(r#"{"ApplicationLatestVersion":"2.2.1.2","FwUpdate0004LatestVersion":"0x0630","FwUpdate000BLatestVersion":"0x0630"}"#.to_string())
    }

    fn download_with_progress(
        &self,
        url: &str,
        _cancel: &CancellationToken,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> dualsense_core::Result<Vec<u8>> {
        assert_eq!(
            url,
            "https://fwupdater.dl.playstation.net/fwupdater/fwupdate000B/0x0630/FWUPDATE000B.bin"
        );
        let total = self.image_bytes.len();
        let half = total / 2;
        on_progress(DownloadProgress {
            downloaded_bytes: half,
            total_bytes: Some(total),
        });
        on_progress(DownloadProgress {
            downloaded_bytes: total,
            total_bytes: Some(total),
        });
        Ok(self.image_bytes.clone())
    }
}

pub fn synthetic_firmware(len: usize, version: u16) -> Vec<u8> {
    let mut bytes: Vec<u8> = (0..len).map(|i| ((i * 31 + 7) & 0xFF) as u8).collect();
    let le = version.to_le_bytes();
    bytes[FIRMWARE_VERSION_OFFSET] = le[0];
    bytes[FIRMWARE_VERSION_OFFSET + 1] = le[1];
    bytes
}

pub fn synthetic_official_firmware(len: usize, pid: u16, target_id: u16, version: u16) -> Vec<u8> {
    let mut bytes = synthetic_firmware(len, version);
    let sig = b"Sony Interactive Entertainment Inc.";
    bytes[..sig.len()].copy_from_slice(sig);
    bytes[FIRMWARE_PID_OFFSET..FIRMWARE_PID_OFFSET + 2].copy_from_slice(&pid.to_le_bytes());
    bytes[FIRMWARE_TARGET_OFFSET..FIRMWARE_TARGET_OFFSET + 2]
        .copy_from_slice(&target_id.to_le_bytes());
    bytes
}
