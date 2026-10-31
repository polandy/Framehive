// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive`, the command line of Framehive (CLI-001). See `README.md`.

#![forbid(unsafe_code)]

mod arguments;

use std::io::{self, Write};
use std::process::ExitCode;

use arguments::Invocation;

/// The exit code when the import did not start (CLI-001).
const NOT_STARTED: u8 = 2;

fn main() -> ExitCode {
    match arguments::parse(std::env::args_os()) {
        Ok(invocation) => run(&invocation),
        Err(error) => exit_for_argument_error(&error),
    }
}

fn run(invocation: &Invocation) -> ExitCode {
    match invocation {
        Invocation::Import(_) => {
            report_error("the import is not available yet");
            ExitCode::from(NOT_STARTED)
        }
    }
}

/// Prints help and version to standard output and argument errors, with a pointer to
/// `--help`, to standard error, and exits with 0 or 2 as clap decides.
fn exit_for_argument_error(error: &clap::Error) -> ExitCode {
    // Nothing more can be reported when the terminal is gone.
    let _ = error.print();
    u8::try_from(error.exit_code()).map_or(ExitCode::from(NOT_STARTED), ExitCode::from)
}

/// Writes an error that stops the command to standard error (CLI-001).
fn report_error(message: &str) {
    // Nothing more can be reported when standard error is gone.
    let _ = writeln!(io::stderr().lock(), "error: {message}");
}
