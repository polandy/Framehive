// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import-log` (CLI-009): prints the import log (IMPORT-010) of the import
//! history in the state folder, which it opens read-only.

mod csv;
mod json;
mod text;

use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use adapter_sqlite::SqliteImportHistory;
use history::{ImportHistory, RunId, RunLog};

use crate::arguments::{ImportLogArguments, LogFormat, RunSelection};
use crate::import_command::process_state_folder;
use crate::{CommandError, NOT_STARTED, report_command_error};

/// The hint after a run that does not exist (CLI-009).
const LIST_RUNS_HINT: &str = "framehive import-log lists the recorded runs";

/// Runs `framehive import-log` and returns its exit code: 0, or 2 with an error on
/// standard error.
pub(crate) fn run_import_log(arguments: ImportLogArguments) -> ExitCode {
    match import_log(arguments) {
        Ok(printed) => {
            // Nothing more can be reported when standard output is gone.
            let _ = io::stdout().lock().write_all(printed.as_bytes());
            ExitCode::SUCCESS
        }
        Err(error) => {
            report_command_error(&error);
            ExitCode::from(NOT_STARTED)
        }
    }
}

/// The text to print.
///
/// # Errors
///
/// Returns the error of CLI-009 for an unknown state folder, a history that cannot
/// be used, or a run that does not exist.
fn import_log(arguments: ImportLogArguments) -> Result<String, CommandError> {
    import_log_in(&process_state_folder()?, arguments)
}

/// The text to print for the import history in `state_folder`.
///
/// # Errors
///
/// Returns the error of CLI-009 for a history that cannot be used, or a run that
/// does not exist.
pub(crate) fn import_log_in(
    state_folder: &Path,
    arguments: ImportLogArguments,
) -> Result<String, CommandError> {
    let history = SqliteImportHistory::read_only(state_folder);
    let Some(selection) = arguments.run else {
        let runs = history.list_runs().map_err(|error| error.to_string())?;
        return Ok(text::run_list(&runs));
    };
    let log = read_log(&history, selection)?;
    Ok(match arguments.format {
        LogFormat::Text => text::run_log(&log),
        LogFormat::Json => json::run_log(&log),
        LogFormat::Csv => csv::run_log(&log),
    })
}

/// The log of the selected run.
///
/// # Errors
///
/// Returns the error for a history that cannot be used or a run that does not exist.
fn read_log(history: &dyn ImportHistory, selection: RunSelection) -> Result<RunLog, CommandError> {
    let run = match selection {
        RunSelection::Number(number) => RunId::new(number),
        RunSelection::Last => history
            .list_runs()
            .map_err(|error| error.to_string())?
            .last()
            .map(|run| run.id)
            .ok_or_else(|| "no import runs recorded".to_owned())?,
    };
    history
        .read_run_log(run)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| CommandError {
            message: format!("no import run {}", run.number()),
            hint: Some(LIST_RUNS_HINT),
        })
}

/// A path as the lines of CLI-001 print it: each byte that is not valid UTF-8 becomes
/// U+FFFD.
fn shown(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod fixtures;
