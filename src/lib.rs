//! Root desktop crate library exposing the Qt bridge, licenses, and snapshot adapter.

pub mod license_report;
pub mod licenses;
pub mod qt_bridge;
pub mod snapshot_adapter;

#[cfg(test)]
mod tests;
