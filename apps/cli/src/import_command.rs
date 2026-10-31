// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` (CLI-001): wires the adapters to an import run.

use std::io;
use std::process::ExitCode;

use adapter_blake3::Blake3Hasher;
use adapter_clock::SystemClock;
use adapter_fs::StdFileSystem;
use import::{ConflictPolicy, ImportRun, ImportSettings, ImportSummary};
use naming::{CompatibilityMode, NameTemplate};

use crate::arguments::{ImportArguments, OnConflict};
use crate::paths::{check_outside, create_destination, resolve_destination, resolve_source};
use crate::report::ReportPrinter;
use crate::{FILES_FAILED, NOT_STARTED, report_error};

/// Runs the import and returns the exit code of CLI-001.
pub(crate) fn run_import(arguments: &ImportArguments) -> ExitCode {
    match import(arguments) {
        Ok(summary) if summary.has_failures() => ExitCode::from(FILES_FAILED),
        Ok(_) => ExitCode::SUCCESS,
        Err(message) => {
            report_error(&message);
            ExitCode::from(NOT_STARTED)
        }
    }
}

/// Checks the arguments, prepares the destination and runs the import, printing the
/// report as it goes.
///
/// # Errors
///
/// Returns the message of the error that kept the import from starting.
fn import(arguments: &ImportArguments) -> Result<ImportSummary, String> {
    let template = NameTemplate::parse(&arguments.template)
        .map_err(|error| format!("invalid name template {}: {error}", arguments.template))?;
    let source = resolve_source(&arguments.source).map_err(|error| error.to_string())?;
    let destination =
        resolve_destination(&arguments.destination).map_err(|error| error.to_string())?;
    check_outside(&source, &destination).map_err(|error| error.to_string())?;

    let file_system = StdFileSystem::new();
    if !arguments.dry_run {
        create_destination(&file_system, &destination).map_err(|error| error.to_string())?;
    }
    let settings = ImportSettings {
        source: &source,
        destination: &destination.path,
        template: &template,
        mode: CompatibilityMode::On,
        policy: conflict_policy(arguments.on_conflict),
        dry_run: arguments.dry_run,
    };
    let clock = SystemClock::new();
    let mut printer = ReportPrinter::new(io::stdout().lock(), arguments.dry_run);
    let summary = ImportRun::new(&file_system, &Blake3Hasher, &clock)
        .run(&settings, &mut printer)
        .map_err(|error| error.to_string())?;
    printer.print_summary(&summary);
    Ok(summary)
}

fn conflict_policy(on_conflict: OnConflict) -> ConflictPolicy {
    match on_conflict {
        OnConflict::AddSuffix => ConflictPolicy::AddSuffix,
        OnConflict::Skip => ConflictPolicy::Skip,
        OnConflict::Fail => ConflictPolicy::Fail,
    }
}
