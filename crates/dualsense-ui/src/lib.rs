//! Headless-testable ViewModel, HTTP client, and async application controller
//! for the DualSense firmware updater GUI.

pub mod error_map;
pub mod http;

pub use error_map::{FriendlyError, map_core_error};
pub use http::{HttpClientConfig, UreqHttpClient, describe_ureq_error};
