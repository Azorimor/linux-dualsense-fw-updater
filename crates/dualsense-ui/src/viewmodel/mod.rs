// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod progress;
pub mod snapshot;
pub mod state;
pub mod transitions;
pub mod types;

pub use progress::{ProgressSnapshot, ProgressStage, format_bytes};
pub use snapshot::UiSnapshot;
pub use state::AppState;
pub use types::{
    AppCommand, AppEvent, AppStep, BannerMessage, BannerSeverity, BusyOperation,
    CachedFirmwareMeta, DeviceInspection, DeviceSummary, DeviceTarget, FirmwareSourceKind,
    IRREVERSIBLE_UPDATE_WARNING, LocalFirmwareInfo, LocalFirmwareSummary, TargetOption,
    UpdateOutcome, UpdateSummary,
};
