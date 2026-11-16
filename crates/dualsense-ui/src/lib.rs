//! Headless-testable ViewModel, HTTP client, and async application controller
//! for the DualSense firmware updater GUI.

pub mod http;

pub use http::{HttpClientConfig, UreqHttpClient, describe_ureq_error};
