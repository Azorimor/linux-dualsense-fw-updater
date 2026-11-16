use std::collections::{BTreeMap, VecDeque};
use std::io::Write as _;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use dualsense_core::{
    CancellationToken, CoreError, DEFAULT_PID, DEFAULT_VID, DUALSENSE_EDGE_PID, DownloadProgress,
    FIRMWARE_INFO_REPORT_LEN, FIRMWARE_VERSION_OFFSET, HidDeviceEnumerator, HidDeviceInfo,
    HidTransport, HttpClient, REPORT_ID_FIRMWARE_INFO, REPORT_ID_SERIAL_RESPONSE,
    REPORT_ID_UPDATE_COMMAND, REPORT_ID_UPDATE_STATUS, Result, SERIAL_REPORT_LEN, Sleeper,
    StartUpdateStatusCode, UpdateCommand, UpdateConfig, VerifyUpdateStatusCode, WRITE_BLOCK_SIZE,
    WriteUpdateStatusCode,
};
use dualsense_ui::{
    AppEvent, AppRuntime, AppStep, BannerSeverity, ProgressStage, UiSnapshot, UreqHttpClient,
};
use tokio::sync::watch;

#[derive(Debug, Clone, Default)]
struct NoopSleeper;

impl Sleeper for NoopSleeper {
    fn sleep(&self, _duration: Duration) {}
}

struct SharedMockTransport {
    sent_reports: Arc<Mutex<Vec<Vec<u8>>>>,
    responses: Arc<Mutex<VecDeque<Result<Vec<u8>>>>>,
    serial_response: Arc<Mutex<Option<Result<Vec<u8>>>>>,
}

impl HidTransport for SharedMockTransport {
    fn get_feature_report(&self, report_id: u8, _length: usize) -> Result<Vec<u8>> {
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
}

type ResponseQueueMap = Mutex<BTreeMap<String, Arc<Mutex<VecDeque<Result<Vec<u8>>>>>>>;
type SerialResponseMap = Mutex<BTreeMap<String, Arc<Mutex<Option<Result<Vec<u8>>>>>>>;
type SentReportsMap = Mutex<BTreeMap<String, Arc<Mutex<Vec<Vec<u8>>>>>>;

struct MockEnumerator {
    list_result: Mutex<Result<Vec<HidDeviceInfo>>>,
    queues: ResponseQueueMap,
    serial_by_path: SerialResponseMap,
    sent_by_path: SentReportsMap,
    open_error: Mutex<Option<CoreError>>,
}

impl Default for MockEnumerator {
    fn default() -> Self {
        Self {
            list_result: Mutex::new(Ok(Vec::new())),
            queues: Mutex::new(BTreeMap::new()),
            serial_by_path: Mutex::new(BTreeMap::new()),
            sent_by_path: Mutex::new(BTreeMap::new()),
            open_error: Mutex::new(None),
        }
    }
}

impl MockEnumerator {
    fn with_devices(devices: Vec<HidDeviceInfo>) -> Self {
        Self {
            list_result: Mutex::new(Ok(devices)),
            ..Default::default()
        }
    }

    fn set_list_result(&self, result: Result<Vec<HidDeviceInfo>>) {
        *self.list_result.lock().expect("lock list") = result;
    }

    fn push_report(&self, path: &str, report: Result<Vec<u8>>) {
        let queue_arc = self
            .queues
            .lock()
            .expect("lock queues")
            .entry(path.to_string())
            .or_default()
            .clone();
        queue_arc.lock().expect("lock queue").push_back(report);
    }

    fn push_firmware_info(&self, path: &str, version: u16) {
        self.push_firmware_info_with_hw(path, version, 0x0004, 0x0000_0411);
    }

    fn push_firmware_info_with_hw(&self, path: &str, version: u16, fw_series: u16, hw_info: u32) {
        self.push_report(
            path,
            Ok(make_firmware_info_report(version, fw_series, hw_info)),
        );
    }

    fn set_serial_number(&self, path: &str, serial: &str) {
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

    fn push_status(&self, path: &str, cmd: UpdateCommand, status_raw: u8) {
        self.push_report(
            path,
            Ok(vec![REPORT_ID_UPDATE_STATUS, cmd as u8, status_raw, 0x00]),
        );
    }

    fn push_full_update_success(&self, path: &str, image_len: usize) {
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

    fn sent_reports(&self, path: &str) -> Vec<Vec<u8>> {
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
            .entry(target_path)
            .or_default()
            .clone();
        Ok(Box::new(SharedMockTransport {
            sent_reports: sent_arc,
            responses: queue,
            serial_response: serial_arc,
        }))
    }
}

struct MockHttp {
    catalog_result: Mutex<Result<String>>,
    firmware_bytes: Mutex<Result<Vec<u8>>>,
    slow_cancel_mode: bool,
}

impl MockHttp {
    fn with_catalog_and_image(catalog_json: &str, image: Vec<u8>) -> Self {
        Self {
            catalog_result: Mutex::new(Ok(catalog_json.to_string())),
            firmware_bytes: Mutex::new(Ok(image)),
            slow_cancel_mode: false,
        }
    }

    fn slow_cancellable(catalog_json: &str) -> Self {
        Self {
            catalog_result: Mutex::new(Ok(catalog_json.to_string())),
            firmware_bytes: Mutex::new(Ok(Vec::new())),
            slow_cancel_mode: true,
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

fn make_device(path: &str, pid: u16, serial: &str) -> HidDeviceInfo {
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

fn make_firmware_info_report(version: u16, fw_series: u16, hw_info: u32) -> Vec<u8> {
    let mut payload64 = vec![0u8; FIRMWARE_INFO_REPORT_LEN];
    payload64[0] = REPORT_ID_FIRMWARE_INFO;
    payload64[1..12].copy_from_slice(b"Oct  1 2024");
    payload64[12..20].copy_from_slice(b"10:20:30");
    payload64[22..24].copy_from_slice(&fw_series.to_le_bytes());
    payload64[24..28].copy_from_slice(&hw_info.to_le_bytes());
    payload64[44..46].copy_from_slice(&version.to_le_bytes());
    payload64
}

fn make_firmware_image_bytes(len: usize, version: u16) -> Vec<u8> {
    let mut bytes = vec![0x5Au8; len];
    let le = version.to_le_bytes();
    bytes[FIRMWARE_VERSION_OFFSET] = le[0];
    bytes[FIRMWARE_VERSION_OFFSET + 1] = le[1];
    bytes
}

fn fast_update_config() -> UpdateConfig {
    UpdateConfig {
        poll_interval: Duration::ZERO,
        max_start_polls: 10,
        max_write_chunk_retries: 10,
        max_verify_polls: 10,
    }
}

fn wait_until<F>(rx: &mut watch::Receiver<UiSnapshot>, predicate: F) -> UiSnapshot
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
            "timed out waiting for UI snapshot condition; last state: step={:?}, busy={}, err={:?}",
            current.step,
            current.is_busy,
            current.error_title
        );
        thread::sleep(Duration::from_millis(5));
    }
}

const SAMPLE_CATALOG_JSON: &str = r#"{"ApplicationLatestVersion":"2.2.1.2","FwUpdate0004LatestVersion":"0x0630","FwUpdate000BLatestVersion":"0x0630","FwUpdate000ELatestVersion":"0x0641","FwUpdate0044LatestVersion":"0x0217"}"#;

#[test]
fn end_to_end_scan_catalog_download_and_update_succeeds() {
    let image_bytes = make_firmware_image_bytes(WRITE_BLOCK_SIZE + 128, 0x0630);
    let enum_mock = Arc::new(MockEnumerator::with_devices(vec![make_device(
        "/dev/hidraw0",
        DEFAULT_PID,
        "ABC123",
    )]));
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);
    enum_mock.push_full_update_success("/dev/hidraw0", image_bytes.len());

    let http_mock = MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, image_bytes.clone());

    let recorded = Arc::new(Mutex::new(Vec::<UiSnapshot>::new()));
    let recorded_cb = Arc::clone(&recorded);

    struct ArcEnumerator(Arc<MockEnumerator>);
    impl HidDeviceEnumerator for ArcEnumerator {
        fn list_devices(&self, vid: u16, pid: u16) -> Result<Vec<HidDeviceInfo>> {
            self.0.list_devices(vid, pid)
        }
        fn open(&self, vid: u16, pid: u16, path: Option<&str>) -> Result<Box<dyn HidTransport>> {
            self.0.open(vid, pid, path)
        }
    }

    let runtime = AppRuntime::with_backends(
        ArcEnumerator(Arc::clone(&enum_mock)),
        http_mock,
        NoopSleeper,
        fast_update_config(),
        move |snap| {
            recorded_cb.lock().expect("lock recorded").push(snap);
        },
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    let after_scan = wait_until(&mut rx, |s| {
        !s.is_busy && s.selected_device.as_ref().and_then(|d| d.firmware_version) == Some(0x0520)
    });
    assert!(after_scan.can_continue_to_firmware);

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::CheckForUpdates);
    let after_catalog = wait_until(&mut rx, |s| !s.is_busy && s.catalog_checked);
    assert!(after_catalog.can_review_update);

    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let completed = wait_until(&mut rx, |s| s.step == AppStep::Completed);
    assert_eq!(completed.completed_version.as_deref(), Some("0x0630"));
    assert_eq!(completed.progress.percentage, Some(100));
    assert_eq!(
        completed.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Positive)
    );

    let history = recorded.lock().expect("lock history");
    assert!(
        history
            .iter()
            .any(|s| s.progress.status_text.starts_with("Downloading firmware…"))
    );
    assert!(
        history
            .iter()
            .any(|s| s.progress.status_text.starts_with("Installing firmware…"))
    );
    let sent = enum_mock.sent_reports("/dev/hidraw0");
    assert!(
        sent.iter()
            .any(|r| r[0] == REPORT_ID_UPDATE_COMMAND && r[1] == UpdateCommand::StartUpdate as u8)
    );
    assert!(
        sent.iter()
            .any(|r| r[0] == REPORT_ID_UPDATE_COMMAND
                && r[1] == UpdateCommand::WriteUpdateImage as u8)
    );
    assert!(sent.iter().any(
        |r| r[0] == REPORT_ID_UPDATE_COMMAND && r[1] == UpdateCommand::VerifyUpdateImage as u8
    ));
    assert_eq!(
        sent.last().map(|r| (r[0], r[1])),
        Some((
            REPORT_ID_UPDATE_COMMAND,
            UpdateCommand::FinalizeUpdate as u8
        ))
    );
    runtime.shutdown();
}

#[test]
fn end_to_end_multi_device_selection_and_local_file_update() {
    let image_bytes = make_firmware_image_bytes(1024, 0x0650);
    let temp_path = std::env::temp_dir().join(format!(
        "dualsense_ui_test_local_{}.bin",
        std::process::id()
    ));
    std::fs::write(&temp_path, &image_bytes).expect("write temp bin");

    let enum_mock = MockEnumerator::with_devices(vec![
        make_device("/dev/hidraw0", DEFAULT_PID, "STD-01"),
        make_device("/dev/hidraw1", DUALSENSE_EDGE_PID, "EDGE-02"),
    ]);
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);
    enum_mock.push_firmware_info_with_hw("/dev/hidraw1", 0x0210, 0x0044, 0x0000_0400);
    enum_mock.push_full_update_success("/dev/hidraw1", image_bytes.len());

    let runtime = AppRuntime::with_backends(
        enum_mock,
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx, |s| {
        !s.is_busy && s.selected_device.as_ref().and_then(|d| d.firmware_version) == Some(0x0520)
    });

    // Switch to second device (DualSense Edge)
    runtime.dispatch(AppEvent::SelectDevice {
        path: "/dev/hidraw1".to_string(),
    });
    let edge_ready = wait_until(&mut rx, |s| {
        !s.is_busy
            && s.selected_device_path.as_deref() == Some("/dev/hidraw1")
            && s.selected_device.as_ref().and_then(|d| d.firmware_version) == Some(0x0210)
    });
    assert_eq!(edge_ready.selected_target_id, "0044");

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::SelectLocalFile {
        path: temp_path.clone(),
    });
    let file_ready = wait_until(&mut rx, |s| !s.is_busy && s.local_firmware.is_some());
    assert!(file_ready.can_review_update);

    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let done = wait_until(&mut rx, |s| s.step == AppStep::Completed);
    assert_eq!(done.completed_version.as_deref(), Some("0x0650"));

    let _ = std::fs::remove_file(&temp_path);
    runtime.shutdown();
}

#[test]
fn end_to_end_download_cancellation_aborts_before_flashing() {
    let enum_mock =
        MockEnumerator::with_devices(vec![make_device("/dev/hidraw0", DEFAULT_PID, "STD-01")]);
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);

    let runtime = AppRuntime::with_backends(
        enum_mock,
        MockHttp::slow_cancellable(SAMPLE_CATALOG_JSON),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::CheckForUpdates);
    wait_until(&mut rx, |s| !s.is_busy && s.catalog_checked);

    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let downloading = wait_until(&mut rx, |s| {
        s.step == AppStep::Updating
            && s.progress.stage == ProgressStage::Downloading
            && s.can_cancel
    });
    assert!(downloading.can_cancel);

    runtime.dispatch(AppEvent::RequestCancel);
    let cancelled = wait_until(&mut rx, |s| s.step == AppStep::ConfirmUpdate && !s.is_busy);
    assert!(!cancelled.warning_acknowledged);
    assert_eq!(
        cancelled.banner.as_ref().map(|b| b.severity),
        Some(BannerSeverity::Info)
    );
    runtime.shutdown();
}

#[test]
fn end_to_end_failure_paths_surface_plain_english_and_recovery() {
    // Case A: USB permission failure on scan
    let enum_mock = MockEnumerator::default();
    enum_mock.set_list_result(Err(CoreError::Hid(
        "Permission denied opening /dev/hidraw0".to_string(),
    )));

    let runtime = AppRuntime::with_backends(
        enum_mock,
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    let err_snap = wait_until(&mut rx, |s| s.step == AppStep::Error);
    assert!(err_snap.error_title.contains("permission"));
    assert!(err_snap.recovery_hint.contains("udev"));
    assert!(err_snap.can_scan);

    runtime.dispatch(AppEvent::DismissError);
    let recovered = wait_until(&mut rx, |s| s.step == AppStep::DetectController);
    assert!(recovered.error_title.is_empty());
    runtime.shutdown();

    // Case B: Controller rejects StartUpdate (already running firmware)
    let enum_mock2 =
        MockEnumerator::with_devices(vec![make_device("/dev/hidraw0", DEFAULT_PID, "STD-01")]);
    enum_mock2.push_firmware_info("/dev/hidraw0", 0x0630);
    enum_mock2.push_status(
        "/dev/hidraw0",
        UpdateCommand::StartUpdate,
        StartUpdateStatusCode::HeaderVersionCheckError as u8,
    );

    let image_bytes = make_firmware_image_bytes(512, 0x0630);
    let runtime2 = AppRuntime::with_backends(
        enum_mock2,
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, image_bytes),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime 2");

    let mut rx2 = runtime2.subscribe();
    runtime2.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx2, |s| !s.is_busy && s.can_continue_to_firmware);
    runtime2.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime2.dispatch(AppEvent::CheckForUpdates);
    wait_until(&mut rx2, |s| !s.is_busy && s.catalog_checked);
    runtime2.dispatch(AppEvent::ContinueToConfirmStep);
    runtime2.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx2, |s| s.can_start_update);

    runtime2.dispatch(AppEvent::StartUpdate);
    let rejection = wait_until(&mut rx2, |s| s.step == AppStep::Error);
    assert!(rejection.error_title.contains("already has this"));
    assert!(!rejection.error_message.contains("HEADER_"));
    assert!(!rejection.recovery_hint.is_empty());
    runtime2.shutdown();
}

#[test]
fn end_to_end_live_ureq_http_client_against_local_server() {
    let image_bytes = make_firmware_image_bytes(1024, 0x0630);
    let image_len = image_bytes.len();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local server");
    let addr = listener.local_addr().expect("local addr");

    let server_handle = thread::spawn(move || {
        use std::io::Read as _;
        let (mut stream, _) = listener.accept().expect("accept download");
        let mut req_buf = [0u8; 512];
        let _ = stream.read(&mut req_buf);
        let headers =
            format!("HTTP/1.1 200 OK\r\nContent-Length: {image_len}\r\nConnection: close\r\n\r\n");
        stream.write_all(headers.as_bytes()).expect("write headers");
        stream.write_all(&image_bytes).expect("write firmware");
    });

    let enum_mock =
        MockEnumerator::with_devices(vec![make_device("/dev/hidraw0", DEFAULT_PID, "STD-01")]);
    enum_mock.push_firmware_info("/dev/hidraw0", 0x0520);
    enum_mock.push_full_update_success("/dev/hidraw0", image_len);

    // Build a catalog pointing at our local TCP HTTP server so real UreqHttpClient downloads it.
    let catalog_json = SAMPLE_CATALOG_JSON.to_string();
    let catalog = dualsense_core::FirmwareCatalog::from_json(&catalog_json).expect("catalog");
    let mut release = catalog
        .release_for_target("0004")
        .expect("0004 release")
        .clone();
    release.download_url = format!("http://{addr}/FWUPDATE0004.bin");

    let runtime = AppRuntime::with_backends(
        enum_mock,
        UreqHttpClient::new(),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);

    let mut local_catalog = catalog;
    if let Some(item) = local_catalog
        .releases
        .iter_mut()
        .find(|r| r.target_id == "0004")
    {
        item.download_url = release.download_url.clone();
    }

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    runtime.dispatch(AppEvent::CatalogLoaded(Ok(local_catalog)));
    wait_until(&mut rx, |s| s.can_review_update);

    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let done = wait_until(&mut rx, |s| s.step == AppStep::Completed);
    assert_eq!(done.completed_version.as_deref(), Some("0x0630"));

    server_handle.join().expect("server thread");
    runtime.shutdown();
}

#[test]
fn end_to_end_bdm050_board_and_midnight_black_color_auto_selects_target() {
    let image_bytes = make_firmware_image_bytes(WRITE_BLOCK_SIZE + 64, 0x0630);
    let enum_mock =
        MockEnumerator::with_devices(vec![make_device("/dev/hidraw0", DEFAULT_PID, "")]);
    enum_mock.push_firmware_info_with_hw("/dev/hidraw0", 0x0520, 0x000B, 0x0000_0811);
    enum_mock.set_serial_number("/dev/hidraw0", "F55301K0H14359961");
    enum_mock.push_full_update_success("/dev/hidraw0", image_bytes.len());

    let http_mock = MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, image_bytes);
    let runtime = AppRuntime::with_backends(
        enum_mock,
        http_mock,
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    let after_scan = wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);

    assert_eq!(after_scan.board_model, "BDM-050");
    assert_eq!(after_scan.fw_series_text, "0x000B");
    assert_eq!(after_scan.hw_model_text, "0x00000811");
    assert_eq!(
        after_scan.serial_number.as_deref(),
        Some("F55301K0H14359961")
    );
    assert_eq!(after_scan.color_name.as_deref(), Some("Midnight Black"));
    assert_eq!(after_scan.detected_target_id.as_deref(), Some("000B"));
    assert!(after_scan.board_supported);
    assert!(after_scan.unsupported_board_warning.is_none());
    assert_eq!(after_scan.selected_target_id, "000B");
    assert!(after_scan.can_select_target);

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    let step2 = wait_until(&mut rx, |s| s.step == AppStep::SelectFirmware);
    assert!(step2.can_select_target);
    assert!(step2.can_check_updates);

    runtime.dispatch(AppEvent::CheckForUpdates);
    let after_catalog = wait_until(&mut rx, |s| !s.is_busy && s.catalog_checked);
    assert!(after_catalog.can_review_update);

    runtime.dispatch(AppEvent::ContinueToConfirmStep);
    runtime.dispatch(AppEvent::SetWarningAcknowledged(true));
    wait_until(&mut rx, |s| s.can_start_update);

    runtime.dispatch(AppEvent::StartUpdate);
    let completed = wait_until(&mut rx, |s| s.step == AppStep::Completed);
    assert_eq!(completed.completed_version.as_deref(), Some("0x0630"));
    runtime.shutdown();
}

#[test]
fn end_to_end_unsupported_bdm010_and_missing_serial_behave_safely() {
    // BDM-010 without 0x80/0x81 serial support: inspection still succeeds and disables catalog update
    let enum_mock =
        MockEnumerator::with_devices(vec![make_device("/dev/hidraw0", DEFAULT_PID, "")]);
    enum_mock.push_firmware_info_with_hw("/dev/hidraw0", 0x0200, 0x0001, 0x0000_0300);

    let runtime = AppRuntime::with_backends(
        enum_mock,
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime");

    let mut rx = runtime.subscribe();
    runtime.dispatch(AppEvent::ScanDevices);
    let after_scan = wait_until(&mut rx, |s| !s.is_busy && s.can_continue_to_firmware);
    assert_eq!(after_scan.board_model, "BDM-010");
    assert_eq!(after_scan.fw_series_text, "0x0001");
    assert_eq!(after_scan.hw_model_text, "0x00000300");
    assert!(after_scan.serial_number.is_none());
    assert!(after_scan.color_name.is_none());
    assert!(after_scan.detected_target_id.is_none());
    assert!(!after_scan.board_supported);
    assert!(!after_scan.can_select_target);
    assert!(
        after_scan
            .unsupported_board_warning
            .as_deref()
            .is_some_and(|w| w.contains("BDM-010"))
    );

    runtime.dispatch(AppEvent::ContinueToFirmwareStep);
    let step2 = wait_until(&mut rx, |s| s.step == AppStep::SelectFirmware);
    assert!(!step2.can_select_target);
    assert!(!step2.can_check_updates);
    assert!(step2.can_select_file);

    runtime.dispatch(AppEvent::SelectTarget {
        target_id: "0004".to_string(),
    });
    runtime.dispatch(AppEvent::CheckForUpdates);
    let guarded = runtime.snapshot();
    assert!(!guarded.is_busy);
    assert!(!guarded.catalog_checked);
    runtime.shutdown();

    // Unknown (zeroed) board on standard PID also disables target selection and update check
    let enum_mock_unknown =
        MockEnumerator::with_devices(vec![make_device("/dev/hidraw0", DEFAULT_PID, "")]);
    enum_mock_unknown.push_firmware_info_with_hw("/dev/hidraw0", 0x0500, 0x0000, 0x0000_0000);

    let runtime_unknown = AppRuntime::with_backends(
        enum_mock_unknown,
        MockHttp::with_catalog_and_image(SAMPLE_CATALOG_JSON, Vec::new()),
        NoopSleeper,
        fast_update_config(),
        |_| {},
    )
    .expect("start runtime unknown");

    let mut rx_unknown = runtime_unknown.subscribe();
    runtime_unknown.dispatch(AppEvent::ScanDevices);
    let snap_unknown = wait_until(&mut rx_unknown, |s| {
        !s.is_busy && s.can_continue_to_firmware
    });
    assert_eq!(snap_unknown.board_model, "Unknown");
    assert!(!snap_unknown.board_supported);
    assert!(!snap_unknown.can_select_target);
    assert!(!snap_unknown.can_check_updates);
    assert!(snap_unknown.unsupported_board_warning.is_some());
    runtime_unknown.shutdown();
}
