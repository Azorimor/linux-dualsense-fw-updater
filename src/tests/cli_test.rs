// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ffi::OsString;

use dualsense_ui::NoFlashOutcome;

use crate::cli::{CliAction, CliOptions, parse_cli_args, usage_text};

fn os_args(items: &[&str]) -> Vec<OsString> {
    items.iter().map(OsString::from).collect()
}

#[test]
fn defaults_with_empty_or_program_only_args() {
    assert_eq!(
        parse_cli_args(Vec::<OsString>::new()),
        Ok(CliAction::Run(CliOptions {
            no_flash: None,
            qt_args: vec!["dualsense-updater".to_string()],
        }))
    );

    assert_eq!(
        parse_cli_args(os_args(&["", "-style", "Fusion"])),
        Ok(CliAction::Run(CliOptions {
            no_flash: None,
            qt_args: vec![
                "dualsense-updater".to_string(),
                "-style".to_string(),
                "Fusion".to_string(),
            ],
        }))
    );

    assert_eq!(
        parse_cli_args(os_args(&["/usr/bin/dualsense-updater"])),
        Ok(CliAction::Run(CliOptions {
            no_flash: None,
            qt_args: vec!["/usr/bin/dualsense-updater".to_string()],
        }))
    );
}

#[test]
fn parses_no_flash_variants_and_strips_them_from_qt_args() {
    assert_eq!(
        parse_cli_args(os_args(&["dualsense-updater", "--no-flash"])),
        Ok(CliAction::Run(CliOptions {
            no_flash: Some(NoFlashOutcome::Success),
            qt_args: vec!["dualsense-updater".to_string()],
        }))
    );

    assert_eq!(
        parse_cli_args(os_args(&[
            "dualsense-updater",
            "-platform",
            "offscreen",
            "--no-flash=success",
            "-style",
            "Fusion",
        ])),
        Ok(CliAction::Run(CliOptions {
            no_flash: Some(NoFlashOutcome::Success),
            qt_args: vec![
                "dualsense-updater".to_string(),
                "-platform".to_string(),
                "offscreen".to_string(),
                "-style".to_string(),
                "Fusion".to_string(),
            ],
        }))
    );

    assert_eq!(
        parse_cli_args(os_args(&[
            "dualsense-updater",
            "--no-flash",
            "success",
            "-platform",
            "offscreen",
        ])),
        Ok(CliAction::Run(CliOptions {
            no_flash: Some(NoFlashOutcome::Success),
            qt_args: vec![
                "dualsense-updater".to_string(),
                "-platform".to_string(),
                "offscreen".to_string(),
            ],
        }))
    );

    assert_eq!(
        parse_cli_args(os_args(&["dualsense-updater", "--no-flash=error"])),
        Ok(CliAction::Run(CliOptions {
            no_flash: Some(NoFlashOutcome::Error),
            qt_args: vec!["dualsense-updater".to_string()],
        }))
    );

    assert_eq!(
        parse_cli_args(os_args(&[
            "dualsense-updater",
            "--no-flash",
            "error",
            "-style",
            "Fusion",
        ])),
        Ok(CliAction::Run(CliOptions {
            no_flash: Some(NoFlashOutcome::Error),
            qt_args: vec![
                "dualsense-updater".to_string(),
                "-style".to_string(),
                "Fusion".to_string(),
            ],
        }))
    );

    assert_eq!(
        parse_cli_args(os_args(&[
            "dualsense-updater",
            "--no-flash",
            "-platform",
            "offscreen",
        ])),
        Ok(CliAction::Run(CliOptions {
            no_flash: Some(NoFlashOutcome::Success),
            qt_args: vec![
                "dualsense-updater".to_string(),
                "-platform".to_string(),
                "offscreen".to_string(),
            ],
        }))
    );
}

#[test]
fn rejects_invalid_no_flash_values_with_clear_error() {
    let err = parse_cli_args(os_args(&["dualsense-updater", "--no-flash=bogus"]))
        .expect_err("invalid --no-flash value must error");
    assert!(err.contains("--no-flash"));
    assert!(err.contains("bogus"));
    assert!(err.contains("success"));
    assert!(err.contains("error"));

    let empty_err = parse_cli_args(os_args(&["dualsense-updater", "--no-flash="]))
        .expect_err("empty --no-flash= value must error");
    assert!(empty_err.contains("--no-flash"));
}

#[test]
fn handles_help_flag_and_double_dash_forwarding() {
    let short_help =
        parse_cli_args(os_args(&["./dualsense-updater", "-h"])).expect("short help succeeds");
    match short_help {
        CliAction::Help(text) => {
            assert!(text.contains("Usage: ./dualsense-updater"));
            assert!(text.contains("--no-flash"));
            assert!(text.contains("--help"));
        }
        CliAction::Run(_) => panic!("expected Help action"),
    }

    let long_help =
        parse_cli_args(os_args(&["app", "--no-flash", "--help"])).expect("long help succeeds");
    assert_eq!(long_help, CliAction::Help(usage_text("app")));

    assert_eq!(
        parse_cli_args(os_args(&["dualsense-updater", "--", "--no-flash"])),
        Ok(CliAction::Run(CliOptions {
            no_flash: None,
            qt_args: vec![
                "dualsense-updater".to_string(),
                "--".to_string(),
                "--no-flash".to_string(),
            ],
        }))
    );
}
