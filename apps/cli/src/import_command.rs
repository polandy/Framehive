// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` (CLI-001): wires the adapters to an import run.

use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use adapter_blake3::Blake3Hasher;
use adapter_clock::SystemClock;
use adapter_fs::StdFileSystem;
use adapter_sequence_counters::FileSequenceCounters;
use adapter_sqlite::SqliteImportHistory;
use import::{
    BackupDestinationError, ConflictPolicy, ImportRun, ImportSettings, ImportSummary, JobCode,
    SessionSequence, check_backup_destinations, require_job_code,
};
use naming::{CompatibilityMode, NameTemplate};

use crate::arguments::{ImportArguments, OnConflict};
use crate::guided::state_folder;
use crate::paths::{
    Destination, PathError, check_outside, create_destination, resolve_destination, resolve_source,
};
use crate::report::{BackupName, ReportPrinter, has_backup_failures};
use crate::{FILES_FAILED, NOT_STARTED, report_error, report_warning};

/// Runs the import and returns the exit code of CLI-001.
pub(crate) fn run_import(arguments: &ImportArguments) -> ExitCode {
    match import(arguments) {
        Ok(summary) if summary.has_failures() || has_backup_failures(&summary) => {
            ExitCode::from(FILES_FAILED)
        }
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
    let job_code = job_code(arguments.job_code.as_deref(), &template)?;
    let source = resolve_source(&arguments.source).map_err(|error| error.to_string())?;
    let destination =
        resolve_destination(&arguments.destination).map_err(|error| error.to_string())?;
    check_outside(&source, &destination).map_err(|error| error.to_string())?;
    let backups = resolve_backups(&arguments.backups)?;
    let backup_paths: Vec<PathBuf> = backups
        .iter()
        .map(|(backup, _)| backup.path.clone())
        .collect();
    check_backup_destinations(&source, &destination.path, &backup_paths)
        .map_err(|error| invalid_backup_message(&error, &backups))?;
    let state_folder = process_state_folder()?;

    let file_system = StdFileSystem::new();
    if !arguments.dry_run {
        create_destination(&file_system, &destination).map_err(|error| error.to_string())?;
        for (backup, given) in &backups {
            create_destination(&file_system, backup)
                .map_err(|error| backup_message(given, &error))?;
        }
    }
    let settings = ImportSettings {
        source: &source,
        destination: &destination.path,
        backup_destinations: &backup_paths,
        template: &template,
        job_code: job_code.as_ref(),
        mode: CompatibilityMode::On,
        policy: conflict_policy(arguments.on_conflict),
        dry_run: arguments.dry_run,
    };
    let clock = SystemClock::new();
    let counters = FileSequenceCounters::new(state_folder.clone());
    let session = SessionSequence::new();
    let history = SqliteImportHistory::new(&state_folder);
    let mut printer = ReportPrinter::new(io::stdout().lock(), arguments.dry_run)
        .with_backup_names(backup_names(&backups));
    let summary = ImportRun::new(
        &file_system,
        &Blake3Hasher,
        &clock,
        &counters,
        &session,
        &history,
    )
    .run(&settings, &mut printer)
    .map_err(|error| error.to_string())?;
    printer.print_summary(&summary);
    if let Some(error) = &summary.history_error {
        report_error(&error.to_string());
    }
    Ok(summary)
}

/// Resolves each backup destination as the destination is resolved, so that the
/// checks of IMPORT-007.11 compare absolute paths without `..` or symbolic links.
///
/// # Errors
///
/// Returns the message for the first backup destination that cannot be resolved.
fn resolve_backups(backups: &[PathBuf]) -> Result<Vec<(Destination, PathBuf)>, String> {
    backups
        .iter()
        .map(|given| {
            resolve_destination(given)
                .map(|backup| (backup, given.clone()))
                .map_err(|error| backup_message(given, &error))
        })
        .collect()
}

/// The backup destinations as the report names them: as given (CLI-005).
fn backup_names(backups: &[(Destination, PathBuf)]) -> Vec<BackupName> {
    backups
        .iter()
        .map(|(backup, given)| BackupName {
            path: backup.path.clone(),
            shown: given.clone(),
        })
        .collect()
}

/// The message for a backup destination that cannot be resolved or created.
fn backup_message(given: &Path, error: &PathError) -> String {
    format!(
        "the backup destination {} cannot be used: {error}",
        given.display()
    )
}

/// The message for a backup destination that IMPORT-007.11 rejects, naming the folder
/// as given.
fn invalid_backup_message(
    error: &BackupDestinationError,
    backups: &[(Destination, PathBuf)],
) -> String {
    let given = backups
        .iter()
        .find(|(backup, _)| backup.path == error.path)
        .map_or(error.path.as_path(), |(_, given)| given.as_path());
    format!(
        "invalid backup destination {}: {}",
        given.display(),
        error.problem
    )
}

/// The state folder of CLI-004 and CLI-008, which holds the persisted sequence
/// counters (ADR 0029) and the import history (ADR 0031).
///
/// # Errors
///
/// Returns the message when the state folder is unknown: `XDG_STATE_HOME` is not an
/// absolute path and the home folder is unknown.
fn process_state_folder() -> Result<PathBuf, String> {
    let home = std::env::home_dir()
        .filter(|home| home.is_absolute())
        .unwrap_or_default();
    let folder = state_folder(std::env::var_os("XDG_STATE_HOME").as_deref(), &home);
    if !folder.is_absolute() {
        return Err("the home folder is unknown; set HOME".to_owned());
    }
    Ok(folder)
}

/// Checks `--job-code` against IMPORT-006 and the template (CLI-001, "Job code"), and
/// warns when the template does not use it.
///
/// # Errors
///
/// Returns the message for an invalid job code, or for a template with `{job}` and no
/// job code.
fn job_code(text: Option<&str>, template: &NameTemplate) -> Result<Option<JobCode>, String> {
    let job_code = text
        .map(|text| {
            JobCode::parse(text).map_err(|error| format!("invalid job code {text:?}: {error}"))
        })
        .transpose()?;
    require_job_code(template, job_code.as_ref())
        .map_err(|error| format!("{error}; give one with --job-code <text>"))?;
    if job_code.is_some() && !template.uses_job_code() {
        report_warning("--job-code is not used, because the name template has no {job}");
    }
    Ok(job_code)
}

fn conflict_policy(on_conflict: OnConflict) -> ConflictPolicy {
    match on_conflict {
        OnConflict::AddSuffix => ConflictPolicy::AddSuffix,
        OnConflict::Skip => ConflictPolicy::Skip,
        OnConflict::Fail => ConflictPolicy::Fail,
    }
}
