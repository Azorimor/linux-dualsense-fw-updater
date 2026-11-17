//! Generated third-party license bundle and pure helper functions.

include!(concat!(env!("OUT_DIR"), "/generated_licenses.rs"));

pub use crate::license_report::{
    CrateMetadata, CrateNotice, LockPackage, format_application_license, format_license_bundle,
    parse_cargo_lock_packages, parse_crate_package_metadata,
};

/// Returns the compile-time generated application and third-party license notice text.
pub fn bundled_licenses_text() -> &'static str {
    BUNDLED_LICENSES_TEXT
}

/// Extracts all `* <crate> v<version> — <license>` summary lines from a license bundle.
pub fn third_party_crate_entries(bundle_text: &str) -> Vec<&str> {
    bundle_text
        .lines()
        .map(str::trim_end)
        .filter(|line| line.starts_with("* ") && line.contains(" — "))
        .collect()
}

/// Returns `true` when the bundle was generated with full third-party license file texts.
pub fn is_full_release_bundle(bundle_text: &str) -> bool {
    !bundle_text.contains("Development build shows crate metadata summary.")
}

/// Returns `true` if `bundle_text` mentions `needle` (case-insensitive).
pub fn contains_notice_for(bundle_text: &str, needle: &str) -> bool {
    let haystack = bundle_text.to_ascii_lowercase();
    let target = needle.to_ascii_lowercase();
    !target.is_empty() && haystack.contains(&target)
}
