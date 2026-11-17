// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use dualsense_core::{
    CoreError, DEFAULT_PID, DUALSENSE_EDGE_PID, FirmwareCatalog, StartUpdateError, UpdateFailure,
};

use super::{sample_hid_device, sample_inspection};
use crate::flash_report::{
    APP_VERSION, FLASH_REPORT_LABEL, FLASH_REPORT_TEMPLATE, GITHUB_NEW_ISSUE_URL,
    SystemEnvironmentInfo, build_flash_report_issue_url, parse_os_release,
    percent_encode_component,
};
use crate::viewmodel::{AppEvent, AppState, AppStep, FirmwareSourceKind, LocalFirmwareInfo};

fn fixture_sys_info() -> SystemEnvironmentInfo {
    SystemEnvironmentInfo {
        os_family: "linux".to_string(),
        distro: "Fedora Linux 43 (KDE Plasma)".to_string(),
        kernel: "6.12.9-200.fc43.x86_64".to_string(),
        arch: "x86_64".to_string(),
        desktop: Some("KDE".to_string()),
        session_type: Some("wayland".to_string()),
        packaging: "AppImage".to_string(),
    }
}

#[test]
fn parse_os_release_handles_pretty_name_fallback_and_quotes() {
    assert_eq!(
        parse_os_release(
            "NAME=\"Ubuntu\"\nVERSION_ID=\"24.04\"\nPRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\n"
        ),
        Some("Ubuntu 24.04.1 LTS".to_string())
    );
    assert_eq!(
        parse_os_release("NAME='Arch Linux'\n"),
        Some("Arch Linux".to_string())
    );
    assert_eq!(
        parse_os_release("NAME=Debian\nVERSION_ID=13\n"),
        Some("Debian 13".to_string())
    );
    assert_eq!(parse_os_release("# empty\nFOO=bar\n"), None);
}

#[test]
fn percent_encode_and_issue_url_encode_reserved_and_utf8_bytes() {
    assert_eq!(percent_encode_component("abc-123_~.Z"), "abc-123_~.Z");
    assert_eq!(
        percent_encode_component("Hello World & = # ? + —"),
        "Hello%20World%20%26%20%3D%20%23%20%3F%20%2B%20%E2%80%94"
    );

    let url = build_flash_report_issue_url("[Flash Report] Test", "Line 1\nLine 2");
    assert!(url.starts_with(GITHUB_NEW_ISSUE_URL));
    assert!(url.contains(&format!("template={FLASH_REPORT_TEMPLATE}")));
    assert!(url.contains(&format!("labels={FLASH_REPORT_LABEL}")));
    assert!(url.contains("title=%5BFlash%20Report%5D%20Test"));
    assert!(url.contains("body=Line%201%0ALine%202"));
}

#[test]
fn flash_report_generated_on_success_and_excludes_serial_and_paths() {
    let mut state = AppState::with_system_info(fixture_sys_info());
    state.update(AppEvent::DevicesScanned(Ok(vec![sample_hid_device(
        "/dev/hidraw99",
        DEFAULT_PID,
        "F32999PRIVATE_SERIAL",
    )])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw99".to_string(),
        result: Ok(sample_inspection(
            0x0482,
            DEFAULT_PID,
            0x000B,
            0x0000_0811,
            Some("F32999PRIVATE_SERIAL"),
        )),
    });
    assert!(state.snapshot().flash_report.is_none());

    state.update(AppEvent::ContinueToFirmwareStep);
    let catalog = FirmwareCatalog::from_json(r#"{"FwUpdate000BLatestVersion":"0x0630"}"#)
        .expect("valid catalog");
    state.update(AppEvent::CatalogLoaded(Ok(catalog)));
    state.update(AppEvent::ContinueToConfirmStep);
    state.update(AppEvent::SetWarningAcknowledged(true));
    state.update(AppEvent::StartUpdate);

    state.update(AppEvent::UpdateFinished(Ok(0x0630)));
    let snap = state.snapshot();
    assert_eq!(snap.step, AppStep::Completed);

    let report = snap.flash_report.expect("flash_report after success");
    assert!(report.succeeded);
    assert!(report.failure_stage.is_none());
    assert!(report.title.starts_with("[Flash Report] Success"));
    assert!(report.title.contains("0x0482 -> 0x0630"));
    assert!(report.markdown.contains("- **Result:** Success"));
    assert!(
        report
            .markdown
            .contains(&format!("- **App Version:** {APP_VERSION}"))
    );
    assert!(report.markdown.contains("- **Board Revision:** BDM-050"));
    assert!(report.markdown.contains("054c:0ce6"));
    assert!(report.markdown.contains("0x00000811"));
    assert!(report.markdown.contains("0x000B"));
    assert!(
        report
            .markdown
            .contains("- **OS / Distribution:** Fedora Linux 43 (KDE Plasma)")
    );
    assert!(
        report
            .markdown
            .contains("- **Kernel:** 6.12.9-200.fc43.x86_64")
    );
    assert!(
        report
            .markdown
            .contains("- **Desktop / Session:** KDE (wayland)")
    );
    assert!(report.markdown.contains("- **Package Format:** AppImage"));
    assert!(
        report
            .markdown
            .contains("- **Resulting Firmware Version:** 0x0630")
    );

    // Strictly no serial number or raw HID path anywhere in report or GitHub URL.
    assert!(!report.title.contains("F32999PRIVATE_SERIAL"));
    assert!(!report.markdown.contains("F32999PRIVATE_SERIAL"));
    assert!(!report.issue_url.contains("F32999PRIVATE_SERIAL"));
    assert!(!report.markdown.contains("/dev/hidraw99"));
    assert!(!report.issue_url.contains("hidraw99"));

    // Resetting clears the report.
    state.update(AppEvent::ResetFlow);
    assert!(state.snapshot().flash_report.is_none());
}

#[test]
fn flash_report_generated_on_update_error_and_local_path_stays_private() {
    let mut state = AppState::with_system_info(fixture_sys_info());
    state.update(AppEvent::DevicesScanned(Ok(vec![sample_hid_device(
        "/dev/hidraw5",
        DUALSENSE_EDGE_PID,
        "G101SECRET_EDGE",
    )])));
    state.update(AppEvent::DeviceFirmwareInfoLoaded {
        path: "/dev/hidraw5".to_string(),
        result: Ok(sample_inspection(
            0x0200,
            DUALSENSE_EDGE_PID,
            0x0044,
            0x0000_4400,
            Some("G101SECRET_EDGE"),
        )),
    });
    state.update(AppEvent::ContinueToFirmwareStep);
    state.update(AppEvent::SetFirmwareSourceKind(
        FirmwareSourceKind::LocalFile,
    ));
    state.update(AppEvent::LocalFileInspected {
        path: PathBuf::from("/home/alice/private/edge_fw_secret.bin"),
        result: Ok(LocalFirmwareInfo {
            version: 0x0250,
            size_bytes: 950_272,
        }),
    });
    state.update(AppEvent::ContinueToConfirmStep);
    state.update(AppEvent::SetWarningAcknowledged(true));
    state.update(AppEvent::StartUpdate);

    let err = CoreError::UpdateFailed(UpdateFailure::StartUpdate(
        StartUpdateError::HeaderVersionCheckError,
    ));
    state.update(AppEvent::UpdateFinished(Err(err)));

    let snap = state.snapshot();
    assert_eq!(snap.step, AppStep::Error);
    let report = snap.flash_report.expect("flash_report after failed flash");
    assert!(!report.succeeded);
    assert_eq!(report.failure_stage.as_deref(), Some("Flashing (USB HID)"));
    assert!(
        report
            .title
            .starts_with("[Flash Report] Failed (Flashing (USB HID))")
    );
    assert!(report.title.contains("DualSense Edge"));
    assert!(report.markdown.contains("### Error Details"));
    assert!(report.markdown.contains("Local .bin firmware file"));
    assert!(
        report
            .markdown
            .contains("- **Untested Hardware Flag:** Yes")
    );

    // Privacy: serial number, username, and local filename must NOT leak.
    assert!(!report.markdown.contains("G101SECRET_EDGE"));
    assert!(!report.markdown.contains("alice"));
    assert!(!report.markdown.contains("edge_fw_secret.bin"));

    // Pre-flash scan errors must NOT expose a flash report, and DismissError clears the report.
    state.update(AppEvent::DismissError);
    assert!(state.snapshot().flash_report.is_none());

    state.update(AppEvent::ResetFlow);
    state.update(AppEvent::DevicesScanned(Err(CoreError::Hid(
        "permission denied".to_string(),
    ))));
    assert_eq!(state.snapshot().step, AppStep::Error);
    assert!(state.snapshot().flash_report.is_none());
}
