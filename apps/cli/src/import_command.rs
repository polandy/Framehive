// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` (CLI-001): wires the adapters to an import run.

use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use adapter_blake3::Blake3Hasher;
use adapter_clock::SystemClock;
use adapter_fs::StdFileSystem;
use adapter_sequence_counters::FileSequenceCounters;
use import::{
    ConflictPolicy, ImportRun, ImportSettings, ImportSummary, JobCode, PlanNotice, SessionSequence,
    require_job_code,
};
use naming::{CompatibilityMode, NameTemplate, SequenceKind};

use crate::arguments::{ImportArguments, OnConflict};
use crate::guided::state_folder;
use crate::paths::{check_outside, create_destination, resolve_destination, resolve_source};
use crate::report::ReportPrinter;
use crate::{FILES_FAILED, NOT_STARTED, report_error, report_warning};

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
    let job_code = job_code(arguments.job_code.as_deref(), &template)?;
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
        backup_destinations: &[],
        template: &template,
        job_code: job_code.as_ref(),
        mode: CompatibilityMode::On,
        policy: conflict_policy(arguments.on_conflict),
        dry_run: arguments.dry_run,
    };
    let clock = SystemClock::new();
    let counters = sequence_counters(&template)?;
    let session = SessionSequence::new();
    let mut printer = ReportPrinter::new(io::stdout().lock(), arguments.dry_run);
    let summary = ImportRun::new(&file_system, &Blake3Hasher, &clock, &counters, &session)
        .run(&settings, &mut printer)
        .map_err(|error| error.to_string())?;
    printer.print_summary(&summary);
    for notice in &summary.notices {
        report_warning(notice_text(*notice));
    }
    Ok(summary)
}

/// The warning for a notice of the plan (CLI-001, "Sequence numbers").
fn notice_text(notice: PlanNotice) -> &'static str {
    match notice {
        PlanNotice::AlreadyImportedNotRecognized => {
            "files already imported are not recognized, because the name template has a persisted sequence counter"
        }
    }
}

/// The persisted sequence counters in the state folder (ADR 0025, ADR 0029).
///
/// # Errors
///
/// Returns the message when the template needs them and the state folder is unknown.
fn sequence_counters(template: &NameTemplate) -> Result<FileSequenceCounters, String> {
    let home = std::env::home_dir().filter(|home| home.is_absolute());
    let needed = template
        .sequence_kind()
        .is_some_and(SequenceKind::is_persisted);
    let home = match home {
        Some(home) => home,
        None if needed => return Err("the home folder is unknown; set HOME".to_owned()),
        // The counters are never read or written without a persisted counter.
        None => PathBuf::new(),
    };
    let folder = state_folder(std::env::var_os("XDG_STATE_HOME").as_deref(), &home);
    Ok(FileSequenceCounters::new(folder))
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
