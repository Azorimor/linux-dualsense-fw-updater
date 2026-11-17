// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Root desktop crate library exposing the Qt bridge, licenses, and snapshot adapter.

pub mod app_icon;
pub mod license_report;
pub mod licenses;
pub mod qt_bridge;
pub mod snapshot_adapter;

#[cfg(test)]
mod tests;
