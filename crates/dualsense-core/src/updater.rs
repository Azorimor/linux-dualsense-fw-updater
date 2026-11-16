use std::time::Duration;

use crate::cancel::CancellationToken;
use crate::error::{
    CoreError, Result, StartUpdateError, UpdateFailure, UpdatePhase, VerifyUpdateImageError,
    WriteUpdateImageError,
};
use crate::image::FirmwareImage;
use crate::protocol::{
    FirmwareInfo, MAX_HID_CHUNK_SIZE, StartUpdateStatusCode, UPDATE_HEADER_LEN,
    UPDATE_STATUS_REPORT_LEN, UpdateCommand, VerifyUpdateStatusCode, WRITE_BLOCK_SIZE,
    WriteUpdateStatusCode,
};
use crate::transport::{DualSenseDevice, HidTransport, Sleeper, StdSleeper};

/// Structured progress snapshot emitted during firmware updates.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateProgress {
    pub phase: UpdatePhase,
    pub current_bytes: usize,
    pub total_bytes: usize,
    pub current_block: usize,
    pub total_blocks: usize,
    pub percentage: f32,
}

impl UpdateProgress {
    fn new(
        phase: UpdatePhase,
        current_bytes: usize,
        total_bytes: usize,
        current_block: usize,
        total_blocks: usize,
    ) -> Self {
        let percentage = if total_bytes == 0 {
            100.0
        } else {
            ((current_bytes as f32 / total_bytes as f32) * 100.0).clamp(0.0, 100.0)
        };
        Self {
            phase,
            current_bytes,
            total_bytes,
            current_block,
            total_blocks,
            percentage,
        }
    }
}

/// Bounded polling and retry limits for [`DualSenseUpdater`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateConfig {
    pub poll_interval: Duration,
    pub max_start_polls: usize,
    pub max_write_chunk_retries: usize,
    pub max_verify_polls: usize,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(10),
            max_start_polls: 1000,
            max_write_chunk_retries: 1000,
            max_verify_polls: 2000,
        }
    }
}

/// High-level DualSense firmware update state machine abstracted over [`HidTransport`] and [`Sleeper`].
#[derive(Debug)]
pub struct DualSenseUpdater<T, S = StdSleeper> {
    dev: DualSenseDevice<T>,
    sleeper: S,
    config: UpdateConfig,
}

impl<T: HidTransport> DualSenseUpdater<T, StdSleeper> {
    /// Creates an updater using [`StdSleeper`] and default polling limits.
    pub fn new(transport: T) -> Self {
        Self::with_sleeper_and_config(transport, StdSleeper, UpdateConfig::default())
    }
}

impl<T: HidTransport, S: Sleeper> DualSenseUpdater<T, S> {
    /// Creates an updater with an injected [`Sleeper`] and [`UpdateConfig`] for deterministic testing.
    pub fn with_sleeper_and_config(transport: T, sleeper: S, config: UpdateConfig) -> Self {
        Self {
            dev: DualSenseDevice::new(transport),
            sleeper,
            config,
        }
    }

    /// Returns a reference to the underlying [`DualSenseDevice`].
    pub fn device(&self) -> &DualSenseDevice<T> {
        &self.dev
    }

    /// Reads the controller's current [`FirmwareInfo`] via Feature Report `0x20`.
    pub fn read_firmware_info(&self) -> Result<FirmwareInfo> {
        self.dev.get_firmware_info()
    }

    /// Executes the `StartUpdate` phase using the 256-byte header of `image`.
    pub fn start_update(&self, image: &FirmwareImage, cancel: &CancellationToken) -> Result<()> {
        check_cancelled(cancel)?;
        let header = image.header();
        if header.len() != UPDATE_HEADER_LEN {
            return Err(CoreError::InvalidUpdateStreamLength(header.len()));
        }

        self.dev
            .send_update_command(UpdateCommand::StartUpdate, header)?;
        let max_attempts = self.config.max_start_polls.max(1);
        for attempt in 1..=max_attempts {
            check_cancelled(cancel)?;
            let status = self.dev.get_update_status(UPDATE_STATUS_REPORT_LEN)?;
            if status.command != UpdateCommand::StartUpdate {
                return Err(CoreError::UnexpectedUpdateStatusCommand {
                    actual: status.command,
                    expected: UpdateCommand::StartUpdate,
                });
            }
            let code = StartUpdateStatusCode::from_int(status.status_raw);
            if matches!(
                code,
                StartUpdateStatusCode::Processing | StartUpdateStatusCode::Retry
            ) {
                if attempt == max_attempts {
                    return Err(CoreError::PollTimeout {
                        phase: UpdatePhase::Starting,
                        attempts: attempt,
                    });
                }
                self.sleeper.sleep(self.config.poll_interval);
                continue;
            }
            if let Some(err) = StartUpdateError::from_status(code) {
                return Err(CoreError::UpdateFailed(UpdateFailure::StartUpdate(err)));
            }
            return Ok(());
        }

        unreachable!("loop returns or errors on max_attempts")
    }

    /// Executes the `WriteUpdateImage` phase, streaming `0x8000`-byte blocks in `0x39`-byte HID chunks.
    pub fn write_update_image<F>(
        &self,
        image: &FirmwareImage,
        cancel: &CancellationToken,
        mut on_progress: F,
    ) -> Result<()>
    where
        F: FnMut(UpdateProgress),
    {
        let total_bytes = image.len();
        let total_blocks = image.block_count();
        let mut bytes_written = 0usize;

        for (block_idx, block) in image.blocks().enumerate() {
            check_cancelled(cancel)?;
            let status = self.write_block(
                block,
                block_idx,
                total_blocks,
                total_bytes,
                &mut bytes_written,
                cancel,
                &mut on_progress,
            )?;
            log::debug!(
                "WriteUpdateImage block {} complete: {} (0x{:02x})",
                block_idx,
                status.name(),
                status as u8
            );
            if let Some(err) = WriteUpdateImageError::from_status(status) {
                return Err(CoreError::UpdateFailed(UpdateFailure::WriteUpdateImage(
                    err,
                )));
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn write_block<F>(
        &self,
        block: &[u8],
        block_idx: usize,
        total_blocks: usize,
        total_bytes: usize,
        bytes_written: &mut usize,
        cancel: &CancellationToken,
        on_progress: &mut F,
    ) -> Result<WriteUpdateStatusCode>
    where
        F: FnMut(UpdateProgress),
    {
        if block.len() > WRITE_BLOCK_SIZE {
            return Err(CoreError::UpdateImageTooLarge(block.len()));
        }
        let max_attempts = self.config.max_write_chunk_retries.max(1);
        let chunks: Vec<&[u8]> = if block.is_empty() {
            vec![&[]]
        } else {
            block.chunks(MAX_HID_CHUNK_SIZE).collect()
        };

        for chunk in chunks {
            check_cancelled(cancel)?;
            self.dev
                .send_update_command(UpdateCommand::WriteUpdateImage, chunk)?;
            for attempt in 1..=max_attempts {
                check_cancelled(cancel)?;
                let status = self.dev.get_update_status(UPDATE_STATUS_REPORT_LEN)?;
                if status.command != UpdateCommand::WriteUpdateImage {
                    return Err(CoreError::UnexpectedUpdateStatusCommand {
                        actual: status.command,
                        expected: UpdateCommand::WriteUpdateImage,
                    });
                }
                let code = WriteUpdateStatusCode::from_int(status.status_raw);
                if matches!(
                    code,
                    WriteUpdateStatusCode::Retry | WriteUpdateStatusCode::AlsoRetry
                ) {
                    if attempt == max_attempts {
                        return Err(CoreError::PollTimeout {
                            phase: UpdatePhase::Writing,
                            attempts: attempt,
                        });
                    }
                    self.sleeper.sleep(self.config.poll_interval);
                    continue;
                }
                if matches!(
                    code,
                    WriteUpdateStatusCode::SendNext | WriteUpdateStatusCode::Success
                ) {
                    *bytes_written += chunk.len();
                    on_progress(UpdateProgress::new(
                        UpdatePhase::Writing,
                        *bytes_written,
                        total_bytes,
                        block_idx + 1,
                        total_blocks,
                    ));
                    break;
                }
                return Ok(code);
            }
        }

        Ok(WriteUpdateStatusCode::Success)
    }

    /// Executes the `VerifyUpdateImage` phase and polls until verification completes.
    pub fn verify_update_image(&self, cancel: &CancellationToken) -> Result<()> {
        check_cancelled(cancel)?;
        self.dev
            .send_update_command(UpdateCommand::VerifyUpdateImage, &[])?;

        let max_attempts = self.config.max_verify_polls.max(1);
        for attempt in 1..=max_attempts {
            check_cancelled(cancel)?;
            let status = self.dev.get_update_status(UPDATE_STATUS_REPORT_LEN)?;
            if status.command != UpdateCommand::VerifyUpdateImage {
                return Err(CoreError::UnexpectedUpdateStatusCommand {
                    actual: status.command,
                    expected: UpdateCommand::VerifyUpdateImage,
                });
            }
            let code = VerifyUpdateStatusCode::from_int(status.status_raw);
            if code == VerifyUpdateStatusCode::KeepPolling {
                if attempt == max_attempts {
                    return Err(CoreError::PollTimeout {
                        phase: UpdatePhase::Verifying,
                        attempts: attempt,
                    });
                }
                self.sleeper.sleep(self.config.poll_interval);
                continue;
            }
            if let Some(err) = VerifyUpdateImageError::from_status(code) {
                return Err(CoreError::UpdateFailed(UpdateFailure::VerifyUpdateImage(
                    err,
                )));
            }
            return Ok(());
        }

        unreachable!("loop returns or errors on max_attempts")
    }

    /// Executes the `FinalizeUpdate` phase (no status polling per DualSense protocol).
    pub fn finalize_update(&self) -> Result<()> {
        self.dev
            .send_update_command(UpdateCommand::FinalizeUpdate, &[])
    }

    /// Runs the full firmware update sequence (`StartUpdate` -> `WriteUpdateImage` -> `VerifyUpdateImage` -> `FinalizeUpdate`).
    pub fn run_full_update<F>(
        &self,
        image: &FirmwareImage,
        cancel: &CancellationToken,
        mut on_progress: F,
    ) -> Result<()>
    where
        F: FnMut(UpdateProgress),
    {
        let total_bytes = image.len();
        let total_blocks = image.block_count();

        on_progress(UpdateProgress::new(
            UpdatePhase::Starting,
            0,
            total_bytes,
            0,
            total_blocks,
        ));
        self.start_update(image, cancel)?;

        on_progress(UpdateProgress::new(
            UpdatePhase::Writing,
            0,
            total_bytes,
            0,
            total_blocks,
        ));
        self.write_update_image(image, cancel, &mut on_progress)?;

        on_progress(UpdateProgress::new(
            UpdatePhase::Verifying,
            total_bytes,
            total_bytes,
            total_blocks,
            total_blocks,
        ));
        self.verify_update_image(cancel)?;

        check_cancelled(cancel)?;
        on_progress(UpdateProgress::new(
            UpdatePhase::Finalizing,
            total_bytes,
            total_bytes,
            total_blocks,
            total_blocks,
        ));
        self.finalize_update()?;

        on_progress(UpdateProgress::new(
            UpdatePhase::Completed,
            total_bytes,
            total_bytes,
            total_blocks,
            total_blocks,
        ));
        Ok(())
    }
}

fn check_cancelled(cancel: &CancellationToken) -> Result<()> {
    if cancel.is_cancelled() {
        Err(CoreError::Cancelled)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use super::*;
    use crate::protocol::{FIRMWARE_VERSION_OFFSET, REPORT_ID_UPDATE_STATUS};

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
        let updater =
            DualSenseUpdater::with_sleeper_and_config(hid, sleeper, UpdateConfig::default());
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
        assert_eq!(sent[1], vec![0xF4, UpdateCommand::FinalizeUpdate as u8, 0x00]);

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
}
