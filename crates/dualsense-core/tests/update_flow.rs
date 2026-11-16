use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Duration;

use dualsense_core::{
    BoardModel, CancellationToken, CoreError, DEFAULT_PID, DEFAULT_VID, DUALSENSE_EDGE_PID,
    DownloadProgress, DualSenseUpdater, FirmwareCatalogClient, FirmwareImage, HidDeviceEnumerator,
    HidDeviceInfo, HidTransport, HttpClient, MAX_HID_CHUNK_SIZE, REPORT_ID_FIRMWARE_INFO,
    REPORT_ID_SERIAL_COMMAND, REPORT_ID_SERIAL_RESPONSE, REPORT_ID_UPDATE_COMMAND,
    REPORT_ID_UPDATE_STATUS, SONY_INFO_JSON_URL, Sleeper, StartUpdateError, StartUpdateStatusCode,
    UPDATE_HEADER_LEN, UpdateCommand, UpdateConfig, UpdateFailure, UpdatePhase, UpdateProgress,
    VerifyUpdateImageError, VerifyUpdateStatusCode, WRITE_BLOCK_SIZE, WriteUpdateImageError,
    WriteUpdateStatusCode, decode_ds5_color, resolve_controller_target,
};

#[derive(Default)]
struct NoopSleeper {
    sleep_count: RefCell<usize>,
}

impl Sleeper for NoopSleeper {
    fn sleep(&self, _duration: Duration) {
        *self.sleep_count.borrow_mut() += 1;
    }
}

/// Stateful simulated DualSense controller validating wire-level `0xF4`/`0xF5`/`0x20`/`0x80`/`0x81` reports.
struct SimDualSenseController {
    current_version: u16,
    fw_series: u16,
    hw_info: u32,
    serial_number: Option<String>,
    received_header: RefCell<Vec<u8>>,
    received_image: RefCell<Vec<u8>>,
    finalized: RefCell<bool>,
    status_queue: RefCell<VecDeque<(UpdateCommand, u8)>>,
    start_processing_polls: usize,
    write_retry_every_n_chunks: usize,
    verify_keep_polling_count: usize,
    written_chunks: RefCell<usize>,
    inject_start_error: Option<StartUpdateStatusCode>,
    inject_write_error_at_chunk: Option<(usize, WriteUpdateStatusCode)>,
    inject_verify_error: Option<VerifyUpdateStatusCode>,
}

impl SimDualSenseController {
    fn new(current_version: u16) -> Self {
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
        }
    }
}

impl HidTransport for SimDualSenseController {
    fn get_feature_report(&self, report_id: u8, length: usize) -> dualsense_core::Result<Vec<u8>> {
        if report_id == REPORT_ID_FIRMWARE_INFO {
            let mut report = vec![0u8; length.max(64)];
            report[0] = REPORT_ID_FIRMWARE_INFO;
            report[1..12].copy_from_slice(b"May 01 2025");
            report[12..20].copy_from_slice(b"14:30:00");
            report[22..24].copy_from_slice(&self.fw_series.to_le_bytes());
            report[24..28].copy_from_slice(&self.hw_info.to_le_bytes());
            let le = self.current_version.to_le_bytes();
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

                if self.write_retry_every_n_chunks > 0
                    && idx.is_multiple_of(self.write_retry_every_n_chunks)
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
}

struct MockEnumerator;

impl HidDeviceEnumerator for MockEnumerator {
    fn list_devices(&self, vid: u16, pid: u16) -> dualsense_core::Result<Vec<HidDeviceInfo>> {
        if vid == DEFAULT_VID && pid == DEFAULT_PID {
            Ok(vec![HidDeviceInfo {
                path: "/dev/hidraw-sim0".to_string(),
                vendor_id: vid,
                product_id: pid,
                interface_number: 3,
                usage_page: 0x0001,
                usage: 0x0005,
                product_string: "DualSense Wireless Controller".to_string(),
                serial_number: "00:11:22:33:44:55".to_string(),
            }])
        } else {
            Ok(Vec::new())
        }
    }

    fn open(
        &self,
        _vid: u16,
        _pid: u16,
        path: Option<&str>,
    ) -> dualsense_core::Result<Box<dyn HidTransport>> {
        if let Some(p) = path
            && p != "/dev/hidraw-sim0"
        {
            return Err(CoreError::DevicePathNotMatched(p.to_string()));
        }
        Ok(Box::new(SimDualSenseController::new(0x0520)))
    }
}

struct MockSonyHttp {
    image_bytes: Vec<u8>,
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

fn synthetic_firmware(len: usize, version: u16) -> Vec<u8> {
    let mut bytes: Vec<u8> = (0..len).map(|i| ((i * 31 + 7) & 0xFF) as u8).collect();
    let le = version.to_le_bytes();
    bytes[0x78] = le[0];
    bytes[0x79] = le[1];
    bytes
}

#[test]
fn end_to_end_catalog_download_and_multi_block_update_succeeds() {
    // 2.5 blocks (> 64 KiB) to verify multi-block 0x8000 splitting and 0x39 chunking.
    let raw_fw = synthetic_firmware(WRITE_BLOCK_SIZE * 2 + 513, 0x0630);
    let catalog_client = FirmwareCatalogClient::new(MockSonyHttp {
        image_bytes: raw_fw.clone(),
    });
    let cancel = CancellationToken::new();

    let catalog = catalog_client
        .fetch_catalog(&cancel)
        .expect("fetch catalog");
    let release = catalog.release_for_target("000B").expect("target 000B");
    let mut dl_events = Vec::new();
    let image = catalog_client
        .download_firmware(release, &cancel, &mut |p| dl_events.push(p))
        .expect("download firmware");
    assert_eq!(dl_events.len(), 2);
    assert_eq!(image.version(), 0x0630);
    assert_eq!(image.block_count(), 3);

    let sim = SimDualSenseController::new(0x0520);
    let updater = DualSenseUpdater::with_sleeper_and_config(
        sim,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );

    let current = updater.read_firmware_info().expect("read firmware info");
    assert_eq!(current.firmware_version, 0x0520);
    assert_eq!(current.formatted_version(), "0x0520");

    let mut progress_log: Vec<UpdateProgress> = Vec::new();
    updater
        .run_full_update(&image, &cancel, |p| progress_log.push(p))
        .expect("full update succeeds");

    let sim_ref = updater.device().transport();
    assert_eq!(sim_ref.received_header.borrow().as_slice(), image.header());
    assert_eq!(sim_ref.received_image.borrow().as_slice(), image.as_bytes());
    assert!(*sim_ref.finalized.borrow());
    assert!(sim_ref.status_queue.borrow().is_empty());

    assert_eq!(progress_log.first().unwrap().phase, UpdatePhase::Starting);
    assert_eq!(progress_log.last().unwrap().phase, UpdatePhase::Completed);
    assert_eq!(progress_log.last().unwrap().percentage, 100.0);
    assert!(
        progress_log
            .windows(2)
            .all(|w| w[0].current_bytes <= w[1].current_bytes)
    );
}

#[test]
fn end_to_end_enumerator_opens_selected_device_and_rejects_unknown_path() {
    let enumerator = MockEnumerator;
    let devices = enumerator.list_dualsense_devices().expect("list devices");
    assert_eq!(devices.len(), 1);

    let transport = enumerator
        .open(DEFAULT_VID, DEFAULT_PID, Some(&devices[0].path))
        .expect("open known device");
    let updater = DualSenseUpdater::new(transport);
    let info = updater.read_firmware_info().expect("firmware info");
    assert_eq!(info.firmware_version, 0x0520);

    let bad = enumerator.open(DEFAULT_VID, DEFAULT_PID, Some("/dev/hidraw-missing"));
    assert!(matches!(bad, Err(CoreError::DevicePathNotMatched(_))));
}

#[test]
fn end_to_end_failure_paths_and_mid_transfer_cancellation() {
    let image = FirmwareImage::from_bytes(synthetic_firmware(1024, 0x0630)).expect("image");

    // 1. Downgrade error in StartUpdate
    let mut sim_downgrade = SimDualSenseController::new(0x0641);
    sim_downgrade.inject_start_error = Some(StartUpdateStatusCode::HeaderVersionCheckError);
    let updater = DualSenseUpdater::with_sleeper_and_config(
        sim_downgrade,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );
    let err = updater
        .run_full_update(&image, &CancellationToken::new(), |_| {})
        .unwrap_err();
    assert_eq!(
        err,
        CoreError::UpdateFailed(UpdateFailure::StartUpdate(
            StartUpdateError::HeaderVersionCheckError
        ))
    );
    assert_eq!(
        err.user_message(),
        "Firmware image is not an upgrade; downgrades are not allowed."
    );

    // 2. Flash write failure mid-write
    let mut sim_write_err = SimDualSenseController::new(0x0520);
    sim_write_err.inject_write_error_at_chunk =
        Some((3, WriteUpdateStatusCode::WriteImageFlashWriteError));
    let updater_write_err = DualSenseUpdater::with_sleeper_and_config(
        sim_write_err,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );
    let err_write = updater_write_err
        .run_full_update(&image, &CancellationToken::new(), |_| {})
        .unwrap_err();
    assert_eq!(
        err_write,
        CoreError::UpdateFailed(UpdateFailure::WriteUpdateImage(
            WriteUpdateImageError::WriteImageFlashWriteError
        ))
    );

    // 3. Body CMAC failure in VerifyUpdateImage
    let mut sim_verify_err = SimDualSenseController::new(0x0520);
    sim_verify_err.inject_verify_error = Some(VerifyUpdateStatusCode::VerifyFwBodyCmacCheckError);
    let updater_verify_err = DualSenseUpdater::with_sleeper_and_config(
        sim_verify_err,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );
    let err_verify = updater_verify_err
        .run_full_update(&image, &CancellationToken::new(), |_| {})
        .unwrap_err();
    assert_eq!(
        err_verify,
        CoreError::UpdateFailed(UpdateFailure::VerifyUpdateImage(
            VerifyUpdateImageError::VerifyFwBodyCmacCheckError
        ))
    );
    assert!(!*updater_verify_err.device().transport().finalized.borrow());

    // 4. Mid-write cancellation triggered by progress callback
    let sim_cancel = SimDualSenseController::new(0x0520);
    let updater_cancel = DualSenseUpdater::with_sleeper_and_config(
        sim_cancel,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let err_cancel = updater_cancel
        .run_full_update(&image, &cancel, |p| {
            if p.phase == UpdatePhase::Writing && p.current_bytes > 0 {
                cancel_clone.cancel();
            }
        })
        .unwrap_err();
    assert_eq!(err_cancel, CoreError::Cancelled);
    assert!(!*updater_cancel.device().transport().finalized.borrow());
}

#[test]
fn end_to_end_board_detection_serial_color_and_unsupported_handling() {
    // 1. Supported BDM-050 controller with 0x80/0x81 serial & colorway detection
    let mut sim_bdm050 = SimDualSenseController::new(0x0520);
    sim_bdm050.fw_series = 0x000B;
    sim_bdm050.hw_info = 0x0000_0811;
    sim_bdm050.serial_number = Some("F12302ABCDEF12345".to_string());

    let updater_050 = DualSenseUpdater::with_sleeper_and_config(
        sim_bdm050,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );
    let info = updater_050.read_firmware_info().expect("firmware info");
    assert_eq!(info.build_date, "May 01 2025");
    assert_eq!(info.board_model(), BoardModel::Bdm050);
    assert_eq!(info.fw_series(), 0x000B);
    assert_eq!(info.hw_info(), 0x0000_0811);

    let hw = resolve_controller_target(&info, DEFAULT_PID);
    assert_eq!(hw.board_name, "BDM-050");
    assert_eq!(hw.supported_target_id, Some("000B"));
    assert!(hw.is_officially_supported);

    let serial = updater_050
        .try_read_serial_number()
        .expect("serial should be decoded");
    assert_eq!(serial, "F12302ABCDEF12345");
    assert_eq!(decode_ds5_color(&serial), Some("Cosmic Red"));

    // Resolve Sony catalog release using auto-detected target ID
    let raw_fw = synthetic_firmware(512, 0x0630);
    let catalog_client = FirmwareCatalogClient::new(MockSonyHttp {
        image_bytes: raw_fw,
    });
    let cancel = CancellationToken::new();
    let catalog = catalog_client.fetch_catalog(&cancel).expect("catalog");
    let release = catalog
        .release_for_target(hw.supported_target_id.unwrap())
        .expect("release");
    let image = catalog_client
        .download_firmware(release, &cancel, &mut |_| {})
        .expect("download");
    updater_050
        .run_full_update(&image, &cancel, |_| {})
        .expect("update succeeds");

    // 2. Unsupported BDM-010 where 0x80/0x81 errors cleanly without breaking read_firmware_info
    let mut sim_bdm010 = SimDualSenseController::new(0x0200);
    sim_bdm010.fw_series = 0x0001;
    sim_bdm010.hw_info = 0x0000_0300;
    sim_bdm010.serial_number = None;

    let updater_010 = DualSenseUpdater::with_sleeper_and_config(
        sim_bdm010,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );
    let info_010 = updater_010
        .read_firmware_info()
        .expect("0x20 succeeds even if 0x80/0x81 unsupported");
    assert_eq!(updater_010.try_read_serial_number(), None);

    let hw_010 = resolve_controller_target(&info_010, DEFAULT_PID);
    assert_eq!(hw_010.board_name, "BDM-010");
    assert_eq!(hw_010.supported_target_id, None);
    assert!(!hw_010.is_officially_supported);
    assert!(hw_010.unsupported_reason.is_some());

    // 3. DualSense Edge resolves to target 0044
    let mut sim_edge = SimDualSenseController::new(0x0217);
    sim_edge.fw_series = 0x0044;
    sim_edge.hw_info = 0x0000_0400;
    let updater_edge = DualSenseUpdater::new(sim_edge);
    let info_edge = updater_edge.read_firmware_info().expect("edge info");
    let hw_edge = resolve_controller_target(&info_edge, DUALSENSE_EDGE_PID);
    assert_eq!(hw_edge.board_name, "DualSense Edge");
    assert_eq!(hw_edge.supported_target_id, Some("0044"));
    assert!(hw_edge.is_officially_supported);
}
