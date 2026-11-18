// SPDX-FileCopyrightText: 2026 nchie
// SPDX-FileCopyrightText: 2026 Azorimor
// SPDX-License-Identifier: MIT AND GPL-3.0-or-later

use std::time::Duration;

use crate::cancel::CancellationToken;
use crate::error::{
    CoreError, Result, StartUpdateError, UpdateFailure, UpdatePhase, VerifyUpdateImageError,
    WriteUpdateImageError,
};
use crate::image::FirmwareImage;
use crate::protocol::{
    BatteryStatus, FirmwareInfo, MAX_HID_CHUNK_SIZE, StartUpdateStatusCode, UPDATE_HEADER_LEN,
    UPDATE_STATUS_REPORT_LEN, UpdateCommand, VerifyUpdateStatusCode, WRITE_BLOCK_SIZE,
    WriteUpdateStatusCode,
};
use crate::transport::{DualSenseDevice, HidTransport, Sleeper, StdSleeper};

/// Default maximum Feature Report `0x20` re-read attempts after `FinalizeUpdate`.
pub const DEFAULT_POST_FLASH_ATTEMPTS: usize = 8;

/// Default delay between post-flash Feature Report `0x20` verification retries.
pub const DEFAULT_POST_FLASH_RETRY_DELAY: Duration = Duration::from_millis(250);

/// Outcome status of best-effort post-flash Feature Report `0x20` verification.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PostFlashVerificationStatus {
    /// Controller responded to `0x20` with the expected new firmware version.
    Verified,
    /// Controller responded to `0x20`, but reported a different firmware version.
    VersionMismatch,
    /// Controller could not be read via `0x20` within the bounded retry budget.
    Unreachable,
}

/// Result of best-effort post-flash hardware verification via Feature Report `0x20`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostFlashVerification {
    pub status: PostFlashVerificationStatus,
    pub expected_version: u16,
    pub firmware_info: Option<FirmwareInfo>,
    pub attempts: usize,
    pub last_error: Option<String>,
}

impl PostFlashVerification {
    /// Returns `true` when the controller confirmed `expected_version` via `0x20`.
    pub fn is_verified(&self) -> bool {
        self.status == PostFlashVerificationStatus::Verified
    }
}

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
    pub(crate) sleeper: S,
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

    /// Best-effort read of the controller serial number via Feature Reports `0x80`/`0x81`.
    pub fn try_read_serial_number(&self) -> Option<String> {
        self.dev.try_read_serial_number()
    }

    /// Best-effort read of the controller battery level and charging state via USB input report `0x01`.
    pub fn try_read_battery_status(&self) -> Option<BatteryStatus> {
        self.dev.try_read_battery_status()
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

    /// Best-effort post-flash check re-reading Feature Report `0x20` using default retry settings.
    pub fn verify_post_flash_firmware(&self, expected_version: u16) -> PostFlashVerification {
        self.verify_post_flash_firmware_with_retry(
            expected_version,
            DEFAULT_POST_FLASH_ATTEMPTS,
            DEFAULT_POST_FLASH_RETRY_DELAY,
        )
    }

    /// Bounded best-effort Feature Report `0x20` re-read after `run_full_update`.
    ///
    /// Tolerates transient USB re-enumeration errors by sleeping `retry_delay` via `self.sleeper`
    /// between attempts, and never fails an already-committed firmware update.
    pub fn verify_post_flash_firmware_with_retry(
        &self,
        expected_version: u16,
        max_attempts: usize,
        retry_delay: Duration,
    ) -> PostFlashVerification {
        let attempts_limit = max_attempts.max(1);
        let mut last_error = None;

        for attempt in 1..=attempts_limit {
            match self.read_firmware_info() {
                Ok(info) => {
                    let status = if info.firmware_version == expected_version {
                        PostFlashVerificationStatus::Verified
                    } else {
                        PostFlashVerificationStatus::VersionMismatch
                    };
                    return PostFlashVerification {
                        status,
                        expected_version,
                        firmware_info: Some(info),
                        attempts: attempt,
                        last_error: None,
                    };
                }
                Err(err) => {
                    last_error = Some(err.technical_detail());
                    if attempt < attempts_limit {
                        self.sleeper.sleep(retry_delay);
                    }
                }
            }
        }

        PostFlashVerification {
            status: PostFlashVerificationStatus::Unreachable,
            expected_version,
            firmware_info: None,
            attempts: attempts_limit,
            last_error,
        }
    }
}

fn check_cancelled(cancel: &CancellationToken) -> Result<()> {
    if cancel.is_cancelled() {
        Err(CoreError::Cancelled)
    } else {
        Ok(())
    }
}
