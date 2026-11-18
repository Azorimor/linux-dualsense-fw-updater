// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT AND GPL-3.0-or-later

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Duration;

use crate::cancel::CancellationToken;
use crate::error::{
    CoreError, Result, StartUpdateError, UpdateFailure, UpdatePhase, VerifyUpdateImageError,
    WriteUpdateImageError,
};
use crate::image::FirmwareImage;
use crate::protocol::{
    BatteryChargingState, BatteryStatus, FIRMWARE_VERSION_OFFSET, INPUT_REPORT_BATTERY_OFFSET,
    INPUT_REPORT_USB_LEN, REPORT_ID_INPUT, REPORT_ID_UPDATE_STATUS, StartUpdateStatusCode,
    UpdateCommand, VerifyUpdateStatusCode, WriteUpdateStatusCode,
};
use crate::transport::{HidTransport, Sleeper};
use crate::updater::*;

#[derive(Default)]
struct MockHid {
    sent: RefCell<Vec<Vec<u8>>>,
    responses: RefCell<VecDeque<Result<Vec<u8>>>>,
}

impl MockHid {
    fn push_status(&self, cmd: UpdateCommand, status_raw: u8) {
        self.responses.borrow_mut().push_back(Ok(vec![
            REPORT_ID_UPDATE_STATUS,
            cmd as u8,
            status_raw,
            0x00,
        ]));
    }

    fn push_raw(&self, data: Result<Vec<u8>>) {
        self.responses.borrow_mut().push_back(data);
    }
}

impl HidTransport for MockHid {
    fn get_feature_report(&self, _report_id: u8, _length: usize) -> Result<Vec<u8>> {
        self.responses
            .borrow_mut()
            .pop_front()
            .expect("mock HID response queue exhausted")
    }

    fn send_feature_report(&self, data: &[u8]) -> Result<()> {
        self.sent.borrow_mut().push(data.to_vec());
        Ok(())
    }
}

#[derive(Default)]
struct RecordingSleeper {
    sleeps: RefCell<Vec<Duration>>,
}

impl Sleeper for RecordingSleeper {
    fn sleep(&self, duration: Duration) {
        self.sleeps.borrow_mut().push(duration);
    }
}

fn test_image(size: usize, version: u16) -> FirmwareImage {
    let mut bytes = vec![0x33u8; size];
    let le = version.to_le_bytes();
    bytes[FIRMWARE_VERSION_OFFSET] = le[0];
    bytes[FIRMWARE_VERSION_OFFSET + 1] = le[1];
    FirmwareImage::from_bytes(bytes).expect("valid test image")
}

#[test]
fn start_update_polls_processing_and_retry_then_succeeds() {
    let hid = MockHid::default();
    hid.push_status(
        UpdateCommand::StartUpdate,
        StartUpdateStatusCode::Processing as u8,
    );
    hid.push_status(
        UpdateCommand::StartUpdate,
        StartUpdateStatusCode::Retry as u8,
    );
    hid.push_status(
        UpdateCommand::StartUpdate,
        StartUpdateStatusCode::Success as u8,
    );

    let sleeper = RecordingSleeper::default();
    let updater = DualSenseUpdater::with_sleeper_and_config(hid, sleeper, UpdateConfig::default());
    let img = test_image(256, 0x0630);
    let cancel = CancellationToken::new();

    updater.start_update(&img, &cancel).expect("start succeeds");
    assert_eq!(updater.sleeper.sleeps.borrow().len(), 2);
    assert_eq!(updater.device().transport().sent.borrow().len(), 5);
}

#[test]
fn start_update_maps_errors_and_timeouts_and_wrong_command() {
    let hid = MockHid::default();
    hid.push_status(
        UpdateCommand::StartUpdate,
        StartUpdateStatusCode::HeaderVersionCheckError as u8,
    );
    let updater = DualSenseUpdater::with_sleeper_and_config(
        hid,
        RecordingSleeper::default(),
        UpdateConfig::default(),
    );
    let img = test_image(256, 0x0630);
    let cancel = CancellationToken::new();

    let err = updater.start_update(&img, &cancel).unwrap_err();
    assert_eq!(
        err,
        CoreError::UpdateFailed(UpdateFailure::StartUpdate(
            StartUpdateError::HeaderVersionCheckError
        ))
    );

    let hid_timeout = MockHid::default();
    hid_timeout.push_status(
        UpdateCommand::StartUpdate,
        StartUpdateStatusCode::Processing as u8,
    );
    hid_timeout.push_status(
        UpdateCommand::StartUpdate,
        StartUpdateStatusCode::Processing as u8,
    );
    let cfg = UpdateConfig {
        max_start_polls: 2,
        ..UpdateConfig::default()
    };
    let updater_timeout =
        DualSenseUpdater::with_sleeper_and_config(hid_timeout, RecordingSleeper::default(), cfg);
    assert_eq!(
        updater_timeout.start_update(&img, &cancel),
        Err(CoreError::PollTimeout {
            phase: UpdatePhase::Starting,
            attempts: 2,
        })
    );

    let hid_wrong = MockHid::default();
    hid_wrong.push_status(UpdateCommand::VerifyUpdateImage, 0x00);
    let updater_wrong = DualSenseUpdater::with_sleeper_and_config(
        hid_wrong,
        RecordingSleeper::default(),
        UpdateConfig::default(),
    );
    assert_eq!(
        updater_wrong.start_update(&img, &cancel),
        Err(CoreError::UnexpectedUpdateStatusCommand {
            actual: UpdateCommand::VerifyUpdateImage,
            expected: UpdateCommand::StartUpdate,
        })
    );
}

#[test]
fn write_update_image_handles_retries_progress_and_errors() {
    // 256 bytes = 5 HID chunks (57*4 + 28)
    let img = test_image(256, 0x0630);
    let hid = MockHid::default();
    // Chunk 0: Retry, AlsoRetry, SendNext
    hid.push_status(
        UpdateCommand::WriteUpdateImage,
        WriteUpdateStatusCode::Retry as u8,
    );
    hid.push_status(
        UpdateCommand::WriteUpdateImage,
        WriteUpdateStatusCode::AlsoRetry as u8,
    );
    hid.push_status(
        UpdateCommand::WriteUpdateImage,
        WriteUpdateStatusCode::SendNext as u8,
    );
    // Chunks 1..=3: SendNext, Chunk 4: Success
    for _ in 0..3 {
        hid.push_status(
            UpdateCommand::WriteUpdateImage,
            WriteUpdateStatusCode::SendNext as u8,
        );
    }
    hid.push_status(
        UpdateCommand::WriteUpdateImage,
        WriteUpdateStatusCode::Success as u8,
    );

    let updater = DualSenseUpdater::with_sleeper_and_config(
        hid,
        RecordingSleeper::default(),
        UpdateConfig::default(),
    );
    let cancel = CancellationToken::new();
    let mut events = Vec::new();
    updater
        .write_update_image(&img, &cancel, |p| events.push(p))
        .expect("write succeeds");

    assert_eq!(events.len(), 5);
    assert_eq!(events[0].current_bytes, 57);
    assert_eq!(events[4].current_bytes, 256);
    assert_eq!(events[4].percentage, 100.0);
    assert_eq!(updater.sleeper.sleeps.borrow().len(), 2);
}

#[test]
fn write_update_image_handles_flash_error_timeout_and_wrong_command() {
    let img = test_image(256, 0x0630);
    let cancel = CancellationToken::new();

    let hid_err = MockHid::default();
    hid_err.push_status(
        UpdateCommand::WriteUpdateImage,
        WriteUpdateStatusCode::WriteImageFlashWriteError as u8,
    );
    let updater_err = DualSenseUpdater::with_sleeper_and_config(
        hid_err,
        RecordingSleeper::default(),
        UpdateConfig::default(),
    );
    assert_eq!(
        updater_err.write_update_image(&img, &cancel, |_| {}),
        Err(CoreError::UpdateFailed(UpdateFailure::WriteUpdateImage(
            WriteUpdateImageError::WriteImageFlashWriteError
        )))
    );

    let hid_timeout = MockHid::default();
    hid_timeout.push_status(
        UpdateCommand::WriteUpdateImage,
        WriteUpdateStatusCode::Retry as u8,
    );
    let updater_timeout = DualSenseUpdater::with_sleeper_and_config(
        hid_timeout,
        RecordingSleeper::default(),
        UpdateConfig {
            max_write_chunk_retries: 1,
            ..UpdateConfig::default()
        },
    );
    assert_eq!(
        updater_timeout.write_update_image(&img, &cancel, |_| {}),
        Err(CoreError::PollTimeout {
            phase: UpdatePhase::Writing,
            attempts: 1,
        })
    );

    let hid_wrong = MockHid::default();
    hid_wrong.push_status(UpdateCommand::StartUpdate, 0x00);
    let updater_wrong = DualSenseUpdater::with_sleeper_and_config(
        hid_wrong,
        RecordingSleeper::default(),
        UpdateConfig::default(),
    );
    assert_eq!(
        updater_wrong.write_update_image(&img, &cancel, |_| {}),
        Err(CoreError::UnexpectedUpdateStatusCommand {
            actual: UpdateCommand::StartUpdate,
            expected: UpdateCommand::WriteUpdateImage,
        })
    );
}

#[test]
fn verify_and_finalize_handle_polling_errors_and_timeouts() {
    let cancel = CancellationToken::new();

    let hid = MockHid::default();
    hid.push_status(
        UpdateCommand::VerifyUpdateImage,
        VerifyUpdateStatusCode::KeepPolling as u8,
    );
    hid.push_status(
        UpdateCommand::VerifyUpdateImage,
        VerifyUpdateStatusCode::Success as u8,
    );
    let updater = DualSenseUpdater::with_sleeper_and_config(
        hid,
        RecordingSleeper::default(),
        UpdateConfig::default(),
    );
    updater.verify_update_image(&cancel).expect("verify ok");
    updater.finalize_update().expect("finalize ok");
    assert_eq!(updater.sleeper.sleeps.borrow().len(), 1);
    let sent = updater.device().transport().sent.borrow();
    assert_eq!(sent.len(), 2);
    assert_eq!(
        sent[1],
        vec![0xF4, UpdateCommand::FinalizeUpdate as u8, 0x00]
    );

    let hid_cmac = MockHid::default();
    hid_cmac.push_status(
        UpdateCommand::VerifyUpdateImage,
        VerifyUpdateStatusCode::VerifyFwBodyCmacCheckError as u8,
    );
    let updater_cmac = DualSenseUpdater::with_sleeper_and_config(
        hid_cmac,
        RecordingSleeper::default(),
        UpdateConfig::default(),
    );
    assert_eq!(
        updater_cmac.verify_update_image(&cancel),
        Err(CoreError::UpdateFailed(UpdateFailure::VerifyUpdateImage(
            VerifyUpdateImageError::VerifyFwBodyCmacCheckError
        )))
    );

    let hid_timeout = MockHid::default();
    hid_timeout.push_status(
        UpdateCommand::VerifyUpdateImage,
        VerifyUpdateStatusCode::KeepPolling as u8,
    );
    let updater_timeout = DualSenseUpdater::with_sleeper_and_config(
        hid_timeout,
        RecordingSleeper::default(),
        UpdateConfig {
            max_verify_polls: 1,
            ..UpdateConfig::default()
        },
    );
    assert_eq!(
        updater_timeout.verify_update_image(&cancel),
        Err(CoreError::PollTimeout {
            phase: UpdatePhase::Verifying,
            attempts: 1,
        })
    );

    let hid_wrong = MockHid::default();
    hid_wrong.push_status(UpdateCommand::WriteUpdateImage, 0x00);
    let updater_wrong = DualSenseUpdater::with_sleeper_and_config(
        hid_wrong,
        RecordingSleeper::default(),
        UpdateConfig::default(),
    );
    assert_eq!(
        updater_wrong.verify_update_image(&cancel),
        Err(CoreError::UnexpectedUpdateStatusCommand {
            actual: UpdateCommand::WriteUpdateImage,
            expected: UpdateCommand::VerifyUpdateImage,
        })
    );
}

#[test]
fn cancellation_stops_update_before_and_during_polling() {
    let img = test_image(256, 0x0630);
    let cancel = CancellationToken::new();
    cancel.cancel();

    let updater = DualSenseUpdater::new(MockHid::default());
    assert_eq!(
        updater.start_update(&img, &cancel),
        Err(CoreError::Cancelled)
    );
    assert_eq!(
        updater.write_update_image(&img, &cancel, |_| {}),
        Err(CoreError::Cancelled)
    );
    assert_eq!(
        updater.verify_update_image(&cancel),
        Err(CoreError::Cancelled)
    );

    // Cancel mid-poll via sleeper side-effect.
    struct CancellingSleeper {
        cancel: CancellationToken,
    }
    impl Sleeper for CancellingSleeper {
        fn sleep(&self, _duration: Duration) {
            self.cancel.cancel();
        }
    }

    let token = CancellationToken::new();
    let hid = MockHid::default();
    hid.push_status(
        UpdateCommand::StartUpdate,
        StartUpdateStatusCode::Processing as u8,
    );
    let mid_updater = DualSenseUpdater::with_sleeper_and_config(
        hid,
        CancellingSleeper {
            cancel: token.clone(),
        },
        UpdateConfig::default(),
    );
    assert_eq!(
        mid_updater.start_update(&img, &token),
        Err(CoreError::Cancelled)
    );
}

#[test]
fn read_firmware_info_and_run_full_update_emit_all_phases() {
    let img = test_image(256, 0x0630);
    let hid = MockHid::default();

    let mut fw_report = vec![0u8; 64];
    fw_report[..11].copy_from_slice(b"Jan 01 2025");
    fw_report[12..20].copy_from_slice(b"12:00:00");
    fw_report[44] = 0x20;
    fw_report[45] = 0x05;
    hid.push_raw(Ok(fw_report));

    hid.push_status(
        UpdateCommand::StartUpdate,
        StartUpdateStatusCode::Success as u8,
    );
    for _ in 0..4 {
        hid.push_status(
            UpdateCommand::WriteUpdateImage,
            WriteUpdateStatusCode::SendNext as u8,
        );
    }
    hid.push_status(
        UpdateCommand::WriteUpdateImage,
        WriteUpdateStatusCode::Success as u8,
    );
    hid.push_status(
        UpdateCommand::VerifyUpdateImage,
        VerifyUpdateStatusCode::Success as u8,
    );

    let updater = DualSenseUpdater::with_sleeper_and_config(
        hid,
        RecordingSleeper::default(),
        UpdateConfig::default(),
    );
    let info = updater.read_firmware_info().expect("read info");
    assert_eq!(info.firmware_version, 0x0520);

    let cancel = CancellationToken::new();
    let mut phases = Vec::new();
    updater
        .run_full_update(&img, &cancel, |p| phases.push(p.phase))
        .expect("full update ok");

    assert_eq!(
        phases,
        vec![
            UpdatePhase::Starting,
            UpdatePhase::Writing,
            UpdatePhase::Writing,
            UpdatePhase::Writing,
            UpdatePhase::Writing,
            UpdatePhase::Writing,
            UpdatePhase::Writing,
            UpdatePhase::Verifying,
            UpdatePhase::Finalizing,
            UpdatePhase::Completed,
        ]
    );
}

#[test]
fn updater_try_read_battery_status_supports_default_none_and_valid_reports() {
    // MockHid does not override read_input_report, so default Ok(None) yields None.
    let updater_none = DualSenseUpdater::new(MockHid::default());
    assert_eq!(updater_none.try_read_battery_status(), None);

    struct BatteryMock(Vec<u8>);
    impl HidTransport for BatteryMock {
        fn get_feature_report(&self, _report_id: u8, _length: usize) -> Result<Vec<u8>> {
            unreachable!()
        }
        fn send_feature_report(&self, _data: &[u8]) -> Result<()> {
            unreachable!()
        }
        fn read_input_report(&self, _timeout_ms: i32) -> Result<Option<Vec<u8>>> {
            Ok(Some(self.0.clone()))
        }
    }

    let mut report = vec![0u8; INPUT_REPORT_USB_LEN];
    report[0] = REPORT_ID_INPUT;
    report[INPUT_REPORT_BATTERY_OFFSET] = 0x2A;
    let updater_full = DualSenseUpdater::new(BatteryMock(report));
    assert_eq!(
        updater_full.try_read_battery_status(),
        Some(BatteryStatus {
            level: 10,
            percentage: 100,
            charging_state: BatteryChargingState::Full,
        })
    );
}
