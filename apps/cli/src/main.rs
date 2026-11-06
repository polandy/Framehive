// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive`, the command line of Framehive (CLI-001, CLI-002, CLI-003, CLI-009, CLI-010). See `README.md`.

#![forbid(unsafe_code)]

mod arguments;
mod config_init;
mod format_readiness;
mod guided;
mod import_command;
mod import_log;
mod paths;
mod profiles;
mod report;

use std::io::{self, Write};
use std::process::ExitCode;

use arguments::Invocation;

/// The exit code when the import ran and a file failed or a folder could not be read
/// (CLI-001).
const FILES_FAILED: u8 = 1;
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
        Invocation::Guided => guided::run_guided(),
        Invocation::Import(arguments) => import_command::run_import(arguments),
        Invocation::ImportLog(arguments) => import_log::run_import_log(*arguments),
        Invocation::ConfigInit => config_init::run_config_init(),
    }
}

/// Prints help and version to standard output and argument errors, with a pointer to
/// `--help`, to standard error, and exits with 0 or 2 as clap decides.
fn exit_for_argument_error(error: &clap::Error) -> ExitCode {
    // Nothing more can be reported when the terminal is gone.
    let _ = error.print();
    u8::try_from(error.exit_code()).map_or(ExitCode::from(NOT_STARTED), ExitCode::from)
}

/// Writes a warning that does not stop the command to standard error (CLI-001).
fn report_warning(message: &str) {
    write_warning(&mut io::stderr().lock(), message);
}

/// Writes a warning that does not stop the command to `errors`, which stands for
/// standard error.
fn write_warning(errors: &mut dyn Write, message: &str) {
    // Nothing more can be reported when standard error is gone.
    let _ = writeln!(errors, "warning: {message}");
}

/// Writes an error that stops the command to standard error (CLI-001).
fn report_error(message: &str) {
    write_error(&mut io::stderr().lock(), message);
}

/// Writes an error to `errors`, which stands for standard error.
fn write_error(errors: &mut dyn Write, message: &str) {
    // Nothing more can be reported when standard error is gone.
    let _ = writeln!(errors, "error: {message}");
}
