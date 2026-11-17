// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use dualsense_core::{DownloadProgress, UpdatePhase, UpdateProgress};

/// Coarse stage used to format progress bars and status labels.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ProgressStage {
    Idle,
    Scanning,
    ReadingController,
    CheckingCatalog,
    InspectingFile,
    Downloading,
    StartingUpdate,
    WritingFirmware,
    VerifyingFirmware,
    FinalizingUpdate,
    Completed,
}

/// Formatted progress state exposed to the presentation layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressSnapshot {
    pub stage: ProgressStage,
    pub percentage: Option<u8>,
    pub indeterminate: bool,
    pub status_text: String,
    pub detail_text: String,
    pub current_bytes: usize,
    pub total_bytes: Option<usize>,
}

impl Default for ProgressSnapshot {
    fn default() -> Self {
        Self::idle()
    }
}

impl ProgressSnapshot {
    pub fn idle() -> Self {
        Self {
            stage: ProgressStage::Idle,
            percentage: None,
            indeterminate: false,
            status_text: String::new(),
            detail_text: String::new(),
            current_bytes: 0,
            total_bytes: None,
        }
    }

    pub fn indeterminate(stage: ProgressStage, status_text: impl Into<String>) -> Self {
        Self {
            stage,
            percentage: None,
            indeterminate: true,
            status_text: status_text.into(),
            detail_text: String::new(),
            current_bytes: 0,
            total_bytes: None,
        }
    }

    pub fn from_download(progress: DownloadProgress) -> Self {
        match progress.total_bytes {
            Some(total) if total > 0 => {
                let pct = compute_percentage(progress.downloaded_bytes, total);
                Self {
                    stage: ProgressStage::Downloading,
                    percentage: Some(pct),
                    indeterminate: false,
                    status_text: format!("Downloading firmware… ({pct}%)"),
                    detail_text: format!(
                        "{} of {}",
                        format_bytes(progress.downloaded_bytes),
                        format_bytes(total)
                    ),
                    current_bytes: progress.downloaded_bytes,
                    total_bytes: Some(total),
                }
            }
            _ => Self {
                stage: ProgressStage::Downloading,
                percentage: None,
                indeterminate: true,
                status_text: "Downloading firmware…".to_string(),
                detail_text: format!("{} downloaded", format_bytes(progress.downloaded_bytes)),
                current_bytes: progress.downloaded_bytes,
                total_bytes: progress.total_bytes,
            },
        }
    }

    pub fn from_update(progress: &UpdateProgress) -> Self {
        match progress.phase {
            UpdatePhase::Starting => Self::indeterminate(
                ProgressStage::StartingUpdate,
                "Preparing controller for update…",
            ),
            UpdatePhase::Writing => {
                let pct = (progress.percentage.round().clamp(0.0, 100.0)) as u8;
                Self {
                    stage: ProgressStage::WritingFirmware,
                    percentage: Some(pct),
                    indeterminate: false,
                    status_text: format!("Installing firmware… ({pct}%)"),
                    detail_text: format!(
                        "{} of {} (part {} of {})",
                        format_bytes(progress.current_bytes),
                        format_bytes(progress.total_bytes),
                        progress.current_block,
                        progress.total_blocks
                    ),
                    current_bytes: progress.current_bytes,
                    total_bytes: Some(progress.total_bytes),
                }
            }
            UpdatePhase::Verifying => Self::indeterminate(
                ProgressStage::VerifyingFirmware,
                "Verifying installed firmware…",
            ),
            UpdatePhase::Finalizing => Self::indeterminate(
                ProgressStage::FinalizingUpdate,
                "Finalizing firmware update…",
            ),
            UpdatePhase::Completed => Self::completed(),
        }
    }

    pub fn completed() -> Self {
        Self {
            stage: ProgressStage::Completed,
            percentage: Some(100),
            indeterminate: false,
            status_text: "Firmware update complete (100%)".to_string(),
            detail_text: String::new(),
            current_bytes: 0,
            total_bytes: None,
        }
    }
}

fn compute_percentage(current: usize, total: usize) -> u8 {
    if total == 0 {
        return 100;
    }
    let ratio = (current as f64 / total as f64) * 100.0;
    ratio.round().clamp(0.0, 100.0) as u8
}

/// Formats a byte count into a human-friendly size string (`B`, `KB`, `MB`).
pub fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
