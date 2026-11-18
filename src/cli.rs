// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Command-line argument parsing for `dualsense-updater`.
//!
//! Extracts application-specific flags such as `--no-flash` and `-h`/`--help`
//! while forwarding remaining Qt arguments (like `-platform` and `-style`) to
//! `QApplication`.

use std::ffi::OsString;

use dualsense_ui::NoFlashOutcome;

const DEFAULT_PROGRAM_NAME: &str = "dualsense-updater";

/// Parsed command-line options used when launching the GUI application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOptions {
    /// Optional `--no-flash` simulation outcome.
    pub no_flash: Option<NoFlashOutcome>,
    /// Program name (`argv[0]`) plus remaining arguments forwarded to Qt.
    pub qt_args: Vec<String>,
}

/// Top-level action produced by [`parse_cli_args`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    /// Print usage text and exit cleanly.
    Help(String),
    /// Run the desktop application with the parsed options.
    Run(CliOptions),
}

/// Formats concise CLI usage instructions for `program_name`.
pub fn usage_text(program_name: &str) -> String {
    let bin = if program_name.trim().is_empty() {
        DEFAULT_PROGRAM_NAME
    } else {
        program_name.trim()
    };
    format!(
        "Usage: {bin} [OPTIONS] [QT_OPTIONS...]\n\
         \n\
         Options:\n\
         \x20     --no-flash[=success|error]\n\
         \x20         Simulate firmware flashing without writing to real hardware.\n\
         \x20         Defaults to 'success' when no outcome is given.\n\
         \x20         Also accepts '--no-flash success' or '--no-flash error'.\n\
         \x20 -h, --help\n\
         \x20         Print this help message and exit.\n"
    )
}

/// Parses process arguments, stripping `--no-flash` flags from `qt_args`.
pub fn parse_cli_args<I>(args: I) -> Result<CliAction, String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut iter = args.into_iter().peekable();
    let program_name = iter
        .next()
        .map(|arg| arg.to_string_lossy().into_owned())
        .filter(|arg| !arg.is_empty())
        .unwrap_or_else(|| DEFAULT_PROGRAM_NAME.to_string());

    let mut no_flash = None;
    let mut qt_args = vec![program_name.clone()];

    while let Some(raw_arg) = iter.next() {
        let arg = raw_arg.to_string_lossy().into_owned();

        if arg == "--" {
            qt_args.push(arg);
            for rest in iter {
                qt_args.push(rest.to_string_lossy().into_owned());
            }
            break;
        }

        if arg == "-h" || arg == "--help" {
            return Ok(CliAction::Help(usage_text(&program_name)));
        }

        if arg == "--no-flash" {
            let next_outcome = iter
                .peek()
                .and_then(|next| parse_no_flash_outcome(&next.to_string_lossy()));
            if let Some(outcome) = next_outcome {
                let _ = iter.next();
                no_flash = Some(outcome);
            } else {
                no_flash = Some(NoFlashOutcome::Success);
            }
            continue;
        }

        if let Some(value) = arg.strip_prefix("--no-flash=") {
            let outcome = parse_no_flash_outcome(value).ok_or_else(|| {
                format!("Invalid value for --no-flash: '{value}'. Expected 'success' or 'error'.")
            })?;
            no_flash = Some(outcome);
            continue;
        }

        qt_args.push(arg);
    }

    Ok(CliAction::Run(CliOptions { no_flash, qt_args }))
}

fn parse_no_flash_outcome(value: &str) -> Option<NoFlashOutcome> {
    match value {
        "success" => Some(NoFlashOutcome::Success),
        "error" => Some(NoFlashOutcome::Error),
        _ => None,
    }
}
