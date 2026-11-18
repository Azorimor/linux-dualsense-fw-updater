// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dualsense_core::{
    BatteryChargingState, BoardModel, CancellationToken, CoreError, DEFAULT_PID, DEFAULT_VID,
    DUALSENSE_EDGE_PID, DualSenseUpdater, FIRMWARE_INFO_REPORT_LEN, FIRMWARE_PID_OFFSET,
    FIRMWARE_TARGET_OFFSET, FIRMWARE_VERSION_OFFSET, FirmwareImage, HidBusType,
    HidDeviceEnumerator, HidDeviceInfo, HidTransport, INPUT_REPORT_BATTERY_OFFSET,
    INPUT_REPORT_USB_LEN, REPORT_ID_FIRMWARE_INFO, REPORT_ID_INPUT, REPORT_ID_SERIAL_RESPONSE,
    REPORT_ID_UPDATE_COMMAND, Result, SERIAL_REPORT_LEN, Sleeper, UpdateCommand, UpdateFailure,
    UpdatePhase, UpdateProgress, VerifyUpdateImageError, decode_ds5_color,
    resolve_controller_target,
};

use crate::no_flash::{
    NoFlashEnumerator, NoFlashOutcome, SIMULATED_DEVICE_PATH, SIMULATED_INITIAL_FW_VERSION,
    SIMULATED_SERIAL_NUMBER, SIMULATED_UPDATE_DURATION, SimulatedDeviceState, run_simulated_update,
};
use crate::viewmodel::AppState;

use super::{sample_bluetooth_hid_device, sample_hid_device};

#[derive(Default)]
struct RecordingSleeper {
    sleeps: Mutex<Vec<Duration>>,
}

impl RecordingSleeper {
    fn total(&self) -> Duration {
        self.sleeps.lock().expect("lock sleeps").iter().sum()
    }
}

impl Sleeper for RecordingSleeper {
    fn sleep(&self, duration: Duration) {
        self.sleeps.lock().expect("lock sleeps").push(duration);
    }
}

struct StubTransport {
    sent: Arc<Mutex<Vec<Vec<u8>>>>,
    responses: Mutex<VecDeque<Vec<u8>>>,
    serial: Vec<u8>,
    battery: Vec<u8>,
}

impl StubTransport {
    fn new(version: u16, serial: &str, battery_byte: u8) -> (Self, Arc<Mutex<Vec<Vec<u8>>>>) {
        let mut fw = vec![0u8; FIRMWARE_INFO_REPORT_LEN];
        fw[0] = REPORT_ID_FIRMWARE_INFO;
        fw[1..12].copy_from_slice(b"Oct  1 2024");
        fw[12..20].copy_from_slice(b"12:00:00");
        fw[22..24].copy_from_slice(&0x0004u16.to_le_bytes());
        fw[24..28].copy_from_slice(&0x0000_0411u32.to_le_bytes());
        fw[44..46].copy_from_slice(&version.to_le_bytes());

        let mut ser = vec![0u8; SERIAL_REPORT_LEN];
        ser[0] = REPORT_ID_SERIAL_RESPONSE;
        ser[1] = 1;
        ser[2] = 19;
        ser[3] = 2;
        let sb = serial.as_bytes();
        ser[4..4 + sb.len().min(17)].copy_from_slice(&sb[..sb.len().min(17)]);

        let mut bat = vec![0u8; INPUT_REPORT_USB_LEN];
        bat[0] = REPORT_ID_INPUT;
        bat[INPUT_REPORT_BATTERY_OFFSET] = battery_byte;

        let sent = Arc::new(Mutex::new(Vec::new()));
        let transport = Self {
            sent: Arc::clone(&sent),
            responses: Mutex::new(VecDeque::from(vec![fw])),
            serial: ser,
            battery: bat,
        };
        (transport, sent)
    }
}

impl HidTransport for StubTransport {
    fn get_feature_report(&self, report_id: u8, _length: usize) -> Result<Vec<u8>> {
        if report_id == REPORT_ID_SERIAL_RESPONSE {
            return Ok(self.serial.clone());
        }
        Ok(self
            .responses
            .lock()
            .expect("lock responses")
            .pop_front()
            .expect("queued response"))
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        self.sent.lock().expect("lock sent").push(data.to_vec());
        Ok(())
    }

    fn read_input_report(&self, _timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        Ok(Some(self.battery.clone()))
    }
}

struct StubEnumerator {
    list_result: Result<Vec<HidDeviceInfo>>,
    real_sent: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl StubEnumerator {
    fn with_result(list_result: Result<Vec<HidDeviceInfo>>) -> Self {
        Self {
            list_result,
            real_sent: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl HidDeviceEnumerator for StubEnumerator {
    fn list_devices(&self, _vid: u16, pid: u16) -> Result<Vec<HidDeviceInfo>> {
        let devices = self.list_result.clone()?;
        Ok(devices
            .into_iter()
            .filter(|d| d.product_id == pid)
            .collect())
    }

    fn open(&self, _vid: u16, _pid: u16, _path: Option<&str>) -> Result<Box<dyn HidTransport>> {
        let (transport, sent) = StubTransport::new(0x0520, "ABC020000000001", 0x05);
        *self.real_sent.lock().expect("lock real_sent") = Vec::new();
        let real_sent = Arc::clone(&self.real_sent);
        struct ForwardingTransport {
            inner: StubTransport,
            real_sent: Arc<Mutex<Vec<Vec<u8>>>>,
        }
        impl HidTransport for ForwardingTransport {
            fn get_feature_report(&self, report_id: u8, length: usize) -> Result<Vec<u8>> {
                self.inner.get_feature_report(report_id, length)
            }
            fn send_feature_report(&self, data: &[u8]) -> Result<()> {
                self.real_sent.lock().expect("lock").push(data.to_vec());
                self.inner.send_feature_report(data)
            }
            fn read_input_report(&self, timeout_ms: i32) -> Result<Option<Vec<u8>>> {
                self.inner.read_input_report(timeout_ms)
            }
        }
        drop(sent);
        Ok(Box::new(ForwardingTransport {
            inner: transport,
            real_sent,
        }))
    }
}

fn make_test_firmware_image(version: u16) -> FirmwareImage {
    let mut bytes = vec![0x5Au8; FirmwareImage::MIN_OFFICIAL_FIRMWARE_BYTES + 1024];
    let sig = b"Sony Interactive Entertainment Inc.";
    bytes[..sig.len()].copy_from_slice(sig);
    bytes[FIRMWARE_PID_OFFSET..FIRMWARE_PID_OFFSET + 2].copy_from_slice(&DEFAULT_PID.to_le_bytes());
    bytes[FIRMWARE_TARGET_OFFSET..FIRMWARE_TARGET_OFFSET + 2]
        .copy_from_slice(&0x000Bu16.to_le_bytes());
    bytes[FIRMWARE_VERSION_OFFSET..FIRMWARE_VERSION_OFFSET + 2]
        .copy_from_slice(&version.to_le_bytes());
    FirmwareImage::from_bytes(bytes).expect("valid firmware image")
}

#[test]
fn no_flash_enumerator_falls_back_to_dummy_when_empty_bluetooth_only_or_hid_error() {
    for inner in [
        StubEnumerator::with_result(Ok(Vec::new())),
        StubEnumerator::with_result(Ok(vec![sample_bluetooth_hid_device(
            "bt://01",
            DEFAULT_PID,
            "BT123",
        )])),
        StubEnumerator::with_result(Err(CoreError::Hid("Permission denied".to_string()))),
    ] {
        let sim = NoFlashEnumerator::new(inner);
        let devices = sim.list_dualsense_devices().expect("dummy fallback");
        assert_eq!(devices.len(), 1);
        let dummy = &devices[0];
        assert_eq!(dummy.path, SIMULATED_DEVICE_PATH);
        assert_eq!(dummy.vendor_id, DEFAULT_VID);
        assert_eq!(dummy.product_id, DEFAULT_PID);
        assert_eq!(dummy.bus_type, HidBusType::Usb);
        assert_eq!(dummy.serial_number, SIMULATED_SERIAL_NUMBER);

        assert_eq!(
            sim.list_devices(DEFAULT_VID, DEFAULT_PID)
                .expect("list std")
                .len(),
            1
        );
        assert!(
            sim.list_devices(DEFAULT_VID, DUALSENSE_EDGE_PID)
                .expect("list edge")
                .is_empty()
        );
    }
}

#[test]
fn no_flash_dummy_transport_exposes_bdm050_serial_color_battery_and_blocks_f4() {
    let sim = NoFlashEnumerator::new(StubEnumerator::with_result(Ok(Vec::new())));
    let transport = sim
        .open(DEFAULT_VID, DEFAULT_PID, Some(SIMULATED_DEVICE_PATH))
        .expect("open simulated device");
    let updater = DualSenseUpdater::new(transport);

    let fw = updater.read_firmware_info().expect("read simulated 0x20");
    assert_eq!(fw.firmware_version, SIMULATED_INITIAL_FW_VERSION);
    assert_eq!(fw.board_model(), BoardModel::Bdm050);

    let hw = resolve_controller_target(&fw, DEFAULT_PID);
    assert_eq!(hw.supported_target_id, Some("000B"));
    assert!(hw.is_officially_supported);

    let serial = updater
        .try_read_serial_number()
        .expect("simulated serial number");
    assert_eq!(serial, SIMULATED_SERIAL_NUMBER);
    assert_eq!(decode_ds5_color(&serial), Some("Midnight Black"));

    let battery = updater
        .try_read_battery_status()
        .expect("simulated battery");
    assert_eq!(battery.charging_state, BatteryChargingState::Charging);
    assert!((70..=95).contains(&battery.percentage));

    let err = updater
        .device()
        .send_update_command(UpdateCommand::StartUpdate, &[0u8; 4])
        .expect_err("0xF4 write must be blocked on simulated transport");
    assert!(matches!(err, CoreError::Hid(_)));
}

#[test]
fn no_flash_real_device_allows_inspection_and_blocks_f4_writes() {
    let real_dev = sample_hid_device("/dev/hidraw9", DEFAULT_PID, "REAL001");
    let inner = StubEnumerator::with_result(Ok(vec![real_dev]));
    let real_sent = Arc::clone(&inner.real_sent);
    let sim = NoFlashEnumerator::new(inner);

    let listed = sim.list_dualsense_devices().expect("list real device");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].path, "/dev/hidraw9");

    let transport = sim
        .open(DEFAULT_VID, DEFAULT_PID, Some("/dev/hidraw9"))
        .expect("open real device");
    let updater = DualSenseUpdater::new(transport);

    let fw = updater.read_firmware_info().expect("read real 0x20");
    assert_eq!(fw.firmware_version, 0x0520);
    assert_eq!(
        updater.try_read_serial_number().as_deref(),
        Some("ABC020000000001")
    );
    let battery = updater.try_read_battery_status().expect("read real 0x01");
    assert_eq!(battery.percentage, 50);

    let err = updater
        .device()
        .send_update_command(UpdateCommand::StartUpdate, &[1, 2, 3, 4])
        .expect_err("0xF4 must never reach real hardware");
    assert!(matches!(err, CoreError::Hid(_)));

    let sent_packets = real_sent.lock().expect("lock real_sent");
    assert!(
        sent_packets
            .iter()
            .all(|pkt| pkt.first().copied() != Some(REPORT_ID_UPDATE_COMMAND)),
        "0xF4 must never be forwarded to inner hardware transport"
    );
}

#[test]
fn run_simulated_update_takes_5_seconds_and_updates_dummy_version_on_success() {
    let image = make_test_firmware_image(0x0630);
    let state = SimulatedDeviceState::default();
    let sleeper = RecordingSleeper::default();
    let cancel = CancellationToken::new();
    let mut events: Vec<UpdateProgress> = Vec::new();

    run_simulated_update(
        NoFlashOutcome::Success,
        &sleeper,
        SIMULATED_DEVICE_PATH,
        Some(&state),
        &image,
        &cancel,
        |progress| events.push(progress),
    )
    .expect("simulated update succeeds");

    assert_eq!(sleeper.total(), SIMULATED_UPDATE_DURATION);
    assert_eq!(state.firmware_version(), 0x0630);
    assert!(events.iter().any(|e| e.phase == UpdatePhase::Starting));
    assert!(events.iter().any(|e| e.phase == UpdatePhase::Writing));
    assert!(events.iter().any(|e| e.phase == UpdatePhase::Verifying));
    assert!(events.iter().any(|e| e.phase == UpdatePhase::Finalizing));
    assert_eq!(events.last().map(|e| e.phase), Some(UpdatePhase::Completed));

    let writing_pcts: Vec<f32> = events
        .iter()
        .filter(|e| e.phase == UpdatePhase::Writing)
        .map(|e| e.percentage)
        .collect();
    assert_eq!(writing_pcts.first().copied(), Some(0.0));
    assert_eq!(writing_pcts.last().copied(), Some(100.0));
    assert!(writing_pcts.windows(2).all(|w| w[0] <= w[1]));
}

#[test]
fn run_simulated_update_error_keeps_dummy_version_and_respects_real_path_and_cancel() {
    let image = make_test_firmware_image(0x0630);
    let state = SimulatedDeviceState::default();
    let sleeper = RecordingSleeper::default();
    let cancel = CancellationToken::new();
    let mut phases = Vec::new();

    let err = run_simulated_update(
        NoFlashOutcome::Error,
        &sleeper,
        SIMULATED_DEVICE_PATH,
        Some(&state),
        &image,
        &cancel,
        |p| phases.push(p.phase),
    )
    .expect_err("NoFlashOutcome::Error must return UpdateFailed");

    assert_eq!(sleeper.total(), SIMULATED_UPDATE_DURATION);
    assert_eq!(state.firmware_version(), SIMULATED_INITIAL_FW_VERSION);
    assert_eq!(
        err,
        CoreError::UpdateFailed(UpdateFailure::VerifyUpdateImage(
            VerifyUpdateImageError::VerifyFwBodyCmacCheckError,
        ))
    );
    assert!(phases.contains(&UpdatePhase::Starting));
    assert!(phases.contains(&UpdatePhase::Writing));
    assert!(phases.contains(&UpdatePhase::Verifying));
    assert!(phases.contains(&UpdatePhase::Finalizing));
    assert!(!phases.contains(&UpdatePhase::Completed));

    // Real device path must not mutate dummy SimulatedDeviceState even on Success.
    let real_sleeper = RecordingSleeper::default();
    run_simulated_update(
        NoFlashOutcome::Success,
        &real_sleeper,
        "/dev/hidraw0",
        Some(&state),
        &image,
        &cancel,
        |_| {},
    )
    .expect("real path simulation succeeds");
    assert_eq!(state.firmware_version(), SIMULATED_INITIAL_FW_VERSION);

    // Pre-cancelled token exits immediately without sleeping.
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    let cancel_sleeper = RecordingSleeper::default();
    let cancel_err = run_simulated_update(
        NoFlashOutcome::Success,
        &cancel_sleeper,
        SIMULATED_DEVICE_PATH,
        Some(&state),
        &image,
        &cancelled,
        |_| {},
    )
    .expect_err("cancelled");
    assert_eq!(cancel_err, CoreError::Cancelled);
    assert_eq!(cancel_sleeper.total(), Duration::ZERO);
}

#[test]
fn app_state_no_flash_flag_only_changes_when_active() {
    let normal = AppState::new().snapshot();
    assert!(!normal.no_flash_active);
    assert_eq!(
        normal.banner.as_ref().map(|b| b.title.as_str()),
        Some("Connect your DualSense controller")
    );

    let simulated = AppState::with_no_flash(true).snapshot();
    assert!(simulated.no_flash_active);
    assert!(
        simulated
            .banner
            .as_ref()
            .is_some_and(|b| b.title.contains("--no-flash"))
    );
}
