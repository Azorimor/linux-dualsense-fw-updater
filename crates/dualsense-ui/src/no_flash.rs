// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! `--no-flash` simulation support for safe end-to-end UI testing without writing firmware to hardware.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use dualsense_core::{
    CancellationToken, CoreError, DEFAULT_PID, DEFAULT_VID, DUALSENSE_EDGE_PID,
    FIRMWARE_INFO_REPORT_LEN, FirmwareImage, HidBusType, HidDeviceEnumerator, HidDeviceInfo,
    HidTransport, INPUT_REPORT_BATTERY_OFFSET, INPUT_REPORT_USB_LEN, REPORT_ID_FIRMWARE_INFO,
    REPORT_ID_INPUT, REPORT_ID_SERIAL_COMMAND, REPORT_ID_SERIAL_RESPONSE, REPORT_ID_UPDATE_COMMAND,
    Result, SERIAL_REPORT_LEN, Sleeper, UpdateFailure, UpdatePhase, UpdateProgress,
    VerifyUpdateImageError, filter_usb_devices,
};

/// Deterministic HID path assigned to the simulated USB DualSense controller.
pub const SIMULATED_DEVICE_PATH: &str = "sim://dualsense/0001";

/// Deterministic serial number for the simulated controller (`"01"` -> Midnight Black).
pub const SIMULATED_SERIAL_NUMBER: &str = "SIM001000000001";

/// Older firmware version reported by the simulated BDM-050 controller so Sony's catalog shows an update.
pub const SIMULATED_INITIAL_FW_VERSION: u16 = 0x0400;

/// Total duration of a simulated firmware update across all phases (`5` seconds).
pub const SIMULATED_UPDATE_DURATION: Duration = Duration::from_secs(5);

const SIMULATED_FW_TYPE: u16 = 0x0003;
const SIMULATED_FW_SERIES_BDM050: u16 = 0x000B;
const SIMULATED_HW_INFO_BDM050: u32 = 0x0000_0713;
const SIMULATED_BUILD_DATE: &[u8; 11] = b"Oct 10 2023";
const SIMULATED_BUILD_TIME: &[u8; 8] = b"14:20:00";
// High nibble 0x1 = Charging, low nibble 0x8 = 80% (~85% charging).
const SIMULATED_BATTERY_BYTE: u8 = 0x18;

const START_SLEEP_MS: u64 = 400;
const WRITE_STEPS: usize = 25;
const WRITE_STEP_SLEEP_MS: u64 = 140; // 25 * 140 = 3500 ms
const VERIFY_SLEEP_MS: u64 = 700;
const FINALIZE_SLEEP_MS: u64 = 400;

/// Outcome simulated by `--no-flash` mode instead of writing firmware to real controller hardware.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum NoFlashOutcome {
    /// Simulates a 5-second update sequence ending in verified completion (Step 4).
    Success,
    /// Simulates a 5-second update sequence ending in a realistic verification failure (Step 5).
    Error,
}

/// Shared mutable firmware version state for the deterministic simulated DualSense controller.
#[derive(Debug)]
pub struct SimulatedDeviceState {
    firmware_version: Mutex<u16>,
}

impl Default for SimulatedDeviceState {
    fn default() -> Self {
        Self::new(SIMULATED_INITIAL_FW_VERSION)
    }
}

impl SimulatedDeviceState {
    /// Creates simulated state initialized to `initial_version`.
    pub fn new(initial_version: u16) -> Self {
        Self {
            firmware_version: Mutex::new(initial_version),
        }
    }

    /// Returns the currently simulated firmware version reported via Feature Report `0x20`.
    pub fn firmware_version(&self) -> u16 {
        *self
            .firmware_version
            .lock()
            .expect("lock simulated firmware version")
    }

    /// Updates the simulated firmware version after a successful `--no-flash` run.
    pub fn set_firmware_version(&self, version: u16) {
        *self
            .firmware_version
            .lock()
            .expect("lock simulated firmware version") = version;
    }
}

/// Returns the deterministic dummy USB DualSense (`BDM-050`, `054c:0ce6`, `Midnight Black`) descriptor.
pub fn simulated_device_info() -> HidDeviceInfo {
    HidDeviceInfo {
        path: SIMULATED_DEVICE_PATH.to_string(),
        vendor_id: DEFAULT_VID,
        product_id: DEFAULT_PID,
        interface_number: 3,
        usage_page: 1,
        usage: 5,
        product_string: "DualSense Wireless Controller".to_string(),
        serial_number: SIMULATED_SERIAL_NUMBER.to_string(),
        bus_type: HidBusType::Usb,
    }
}

/// [`HidDeviceEnumerator`] wrapper used when `--no-flash` simulation mode is active.
///
/// Queries the real enumerator first so physically attached USB DualSense controllers can still be
/// inspected safely (with `0xF4` update commands blocked), and falls back to a deterministic dummy
/// USB DualSense (`BDM-050`) when no USB DualSense is found or HID enumeration fails.
#[derive(Debug)]
pub struct NoFlashEnumerator<E> {
    inner: E,
    simulated_state: Arc<SimulatedDeviceState>,
}

impl<E> NoFlashEnumerator<E> {
    /// Wraps `inner` with fresh simulated device state initialized to `0x0400`.
    pub fn new(inner: E) -> Self {
        Self::with_shared_state(inner, Arc::new(SimulatedDeviceState::default()))
    }

    /// Wraps `inner` with an explicit shared [`SimulatedDeviceState`].
    pub fn with_shared_state(inner: E, simulated_state: Arc<SimulatedDeviceState>) -> Self {
        Self {
            inner,
            simulated_state,
        }
    }

    /// Returns a handle to the shared simulated controller state.
    pub fn simulated_state(&self) -> Arc<SimulatedDeviceState> {
        Arc::clone(&self.simulated_state)
    }

    /// Returns a reference to the wrapped [`HidDeviceEnumerator`].
    pub fn inner(&self) -> &E {
        &self.inner
    }
}

impl<E: HidDeviceEnumerator> HidDeviceEnumerator for NoFlashEnumerator<E> {
    fn list_devices(&self, vid: u16, pid: u16) -> Result<Vec<HidDeviceInfo>> {
        if vid == DEFAULT_VID && (pid == DEFAULT_PID || pid == DUALSENSE_EDGE_PID) {
            let devices = self.list_dualsense_devices()?;
            Ok(devices
                .into_iter()
                .filter(|d| d.vendor_id == vid && d.product_id == pid)
                .collect())
        } else {
            self.inner.list_devices(vid, pid)
        }
    }

    fn open(&self, vid: u16, pid: u16, path: Option<&str>) -> Result<Box<dyn HidTransport>> {
        if path == Some(SIMULATED_DEVICE_PATH) {
            return Ok(Box::new(SimulatedTransport {
                state: Arc::clone(&self.simulated_state),
            }));
        }

        match self.inner.open(vid, pid, path) {
            Ok(transport) => Ok(Box::new(ReadOnlyGuardTransport { inner: transport })),
            Err(err) if path.is_none() && vid == DEFAULT_VID && pid == DEFAULT_PID => {
                log::debug!(
                    "HID open(None) failed in --no-flash mode ({err}); falling back to simulated device"
                );
                Ok(Box::new(SimulatedTransport {
                    state: Arc::clone(&self.simulated_state),
                }))
            }
            Err(err) => Err(err),
        }
    }

    fn list_dualsense_devices(&self) -> Result<Vec<HidDeviceInfo>> {
        match self.inner.list_dualsense_devices() {
            Ok(devices) if !filter_usb_devices(&devices).is_empty() => Ok(devices),
            Ok(_) => Ok(vec![simulated_device_info()]),
            Err(err) => {
                log::debug!(
                    "HID enumeration failed in --no-flash mode ({err}); returning simulated DualSense"
                );
                Ok(vec![simulated_device_info()])
            }
        }
    }
}

struct SimulatedTransport {
    state: Arc<SimulatedDeviceState>,
}

impl HidTransport for SimulatedTransport {
    fn get_feature_report(&self, report_id: u8, _length: usize) -> Result<Vec<u8>> {
        match report_id {
            REPORT_ID_FIRMWARE_INFO => Ok(build_simulated_firmware_info_report(
                self.state.firmware_version(),
            )),
            REPORT_ID_SERIAL_RESPONSE => Ok(build_simulated_serial_report()),
            other => Err(CoreError::Hid(format!(
                "Simulated DualSense transport does not support feature report 0x{other:02X}."
            ))),
        }
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        match data.first().copied() {
            Some(REPORT_ID_SERIAL_COMMAND) => Ok(()),
            Some(REPORT_ID_UPDATE_COMMAND) => Err(CoreError::Hid(
                "Blocked 0xF4 firmware update command on simulated DualSense transport."
                    .to_string(),
            )),
            _ => Err(CoreError::Hid(
                "Simulated DualSense transport rejected unsupported feature report write."
                    .to_string(),
            )),
        }
    }

    fn read_input_report(&self, _timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        let mut raw = vec![0u8; INPUT_REPORT_USB_LEN];
        raw[0] = REPORT_ID_INPUT;
        raw[INPUT_REPORT_BATTERY_OFFSET] = SIMULATED_BATTERY_BYTE;
        Ok(Some(raw))
    }
}

struct ReadOnlyGuardTransport {
    inner: Box<dyn HidTransport>,
}

impl HidTransport for ReadOnlyGuardTransport {
    fn get_feature_report(&self, report_id: u8, length: usize) -> Result<Vec<u8>> {
        self.inner.get_feature_report(report_id, length)
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        match data.first().copied() {
            Some(REPORT_ID_SERIAL_COMMAND) => self.inner.send_feature_report(data),
            Some(REPORT_ID_UPDATE_COMMAND) => Err(CoreError::Hid(
                "Blocked 0xF4 firmware update command: hardware writes are disabled in --no-flash mode."
                    .to_string(),
            )),
            _ => Err(CoreError::Hid(
                "Blocked non-inspection feature report write in --no-flash mode.".to_string(),
            )),
        }
    }

    fn read_input_report(&self, timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        self.inner.read_input_report(timeout_ms)
    }
}

fn build_simulated_firmware_info_report(firmware_version: u16) -> Vec<u8> {
    let mut raw = vec![0u8; FIRMWARE_INFO_REPORT_LEN];
    raw[0] = REPORT_ID_FIRMWARE_INFO;
    raw[1..12].copy_from_slice(SIMULATED_BUILD_DATE);
    raw[12..20].copy_from_slice(SIMULATED_BUILD_TIME);
    raw[20..22].copy_from_slice(&SIMULATED_FW_TYPE.to_le_bytes());
    raw[22..24].copy_from_slice(&SIMULATED_FW_SERIES_BDM050.to_le_bytes());
    raw[24..28].copy_from_slice(&SIMULATED_HW_INFO_BDM050.to_le_bytes());
    raw[28..32].copy_from_slice(&(firmware_version as u32).to_le_bytes());
    raw[44..46].copy_from_slice(&firmware_version.to_le_bytes());
    raw
}

fn build_simulated_serial_report() -> Vec<u8> {
    let mut raw = vec![0u8; SERIAL_REPORT_LEN];
    raw[0] = REPORT_ID_SERIAL_RESPONSE;
    raw[1] = 1;
    raw[2] = 19;
    raw[3] = 2;
    let bytes = SIMULATED_SERIAL_NUMBER.as_bytes();
    let copy_len = bytes.len().min(17);
    raw[4..4 + copy_len].copy_from_slice(&bytes[..copy_len]);
    raw
}

/// Runs a 5-second simulated firmware update progress sequence without touching hardware.
///
/// Uses `sleeper` for every phase delay so tests can inject a no-op or recording [`Sleeper`] and
/// run deterministically without real wall-clock delays.
pub fn run_simulated_update<S, F>(
    outcome: NoFlashOutcome,
    sleeper: &S,
    device_path: &str,
    simulated_state: Option<&SimulatedDeviceState>,
    image: &FirmwareImage,
    cancel: &CancellationToken,
    mut on_progress: F,
) -> Result<()>
where
    S: Sleeper,
    F: FnMut(UpdateProgress),
{
    let total_bytes = image.len();
    let total_blocks = image.block_count();

    check_cancelled(cancel)?;
    on_progress(make_progress(
        UpdatePhase::Starting,
        0,
        total_bytes,
        0,
        total_blocks,
        0.0,
    ));
    sleeper.sleep(Duration::from_millis(START_SLEEP_MS));

    check_cancelled(cancel)?;
    on_progress(make_progress(
        UpdatePhase::Writing,
        0,
        total_bytes,
        0,
        total_blocks,
        0.0,
    ));
    for step in 1..=WRITE_STEPS {
        check_cancelled(cancel)?;
        sleeper.sleep(Duration::from_millis(WRITE_STEP_SLEEP_MS));
        let current_bytes = (total_bytes * step) / WRITE_STEPS;
        let current_block = if total_blocks == 0 {
            0
        } else {
            ((total_blocks * step) / WRITE_STEPS).clamp(1, total_blocks)
        };
        let percentage = ((step as f32 / WRITE_STEPS as f32) * 100.0).clamp(0.0, 100.0);
        on_progress(make_progress(
            UpdatePhase::Writing,
            current_bytes,
            total_bytes,
            current_block,
            total_blocks,
            percentage,
        ));
    }

    check_cancelled(cancel)?;
    on_progress(make_progress(
        UpdatePhase::Verifying,
        total_bytes,
        total_bytes,
        total_blocks,
        total_blocks,
        100.0,
    ));
    sleeper.sleep(Duration::from_millis(VERIFY_SLEEP_MS));

    check_cancelled(cancel)?;
    on_progress(make_progress(
        UpdatePhase::Finalizing,
        total_bytes,
        total_bytes,
        total_blocks,
        total_blocks,
        100.0,
    ));
    sleeper.sleep(Duration::from_millis(FINALIZE_SLEEP_MS));

    match outcome {
        NoFlashOutcome::Success => {
            if device_path == SIMULATED_DEVICE_PATH
                && let Some(state) = simulated_state
            {
                state.set_firmware_version(image.version());
            }
            on_progress(make_progress(
                UpdatePhase::Completed,
                total_bytes,
                total_bytes,
                total_blocks,
                total_blocks,
                100.0,
            ));
            Ok(())
        }
        NoFlashOutcome::Error => Err(CoreError::UpdateFailed(UpdateFailure::VerifyUpdateImage(
            VerifyUpdateImageError::VerifyFwBodyCmacCheckError,
        ))),
    }
}

fn make_progress(
    phase: UpdatePhase,
    current_bytes: usize,
    total_bytes: usize,
    current_block: usize,
    total_blocks: usize,
    percentage: f32,
) -> UpdateProgress {
    UpdateProgress {
        phase,
        current_bytes,
        total_bytes,
        current_block,
        total_blocks,
        percentage,
    }
}

fn check_cancelled(cancel: &CancellationToken) -> Result<()> {
    if cancel.is_cancelled() {
        Err(CoreError::Cancelled)
    } else {
        Ok(())
    }
}
