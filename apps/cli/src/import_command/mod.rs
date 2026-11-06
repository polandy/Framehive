// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` (CLI-001, CLI-010): wires the adapters to an import run.

mod folders;
mod run;
#[cfg(test)]
mod tests;

use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use adapter_free_space::StatvfsFreeSpace;
use adapter_fs::StdFileSystem;
use adapter_yaml::SaphyrParser;
use import::{
    ConflictPolicy, DEFAULT_NAME_TEMPLATE, ImportProfile, ImportSummary, JobCode, require_job_code,
};
use naming::NameTemplate;

use crate::arguments::{ImportArguments, OnConflict};
use crate::guided::state_folder;
use crate::profiles::{
    check_profile_name, process_configuration_folder, process_home_folder, profiles_folder,
    read_profile,
};
use crate::report::has_backup_failures;
use crate::{FILES_FAILED, NOT_STARTED, report_error, report_warning};
use run::{NotStarted, Surroundings, import_with};

/// Runs the import and returns the exit code of CLI-001.
pub(crate) fn run_import(arguments: &ImportArguments) -> ExitCode {
    match import(arguments) {
        Ok(summary) if summary.has_failures() || has_backup_failures(&summary) => {
            ExitCode::from(FILES_FAILED)
        }
        Ok(_) => ExitCode::SUCCESS,
        Err(NotStarted(messages)) => {
            for message in &messages {
                report_error(message);
            }
            ExitCode::from(NOT_STARTED)
        }
    }
}

/// Reads the profile and the state folder from the process environment, and imports
/// with the real free space, standard output and standard error.
///
/// # Errors
///
/// Returns the messages of the errors that kept the import from starting.
fn import(arguments: &ImportArguments) -> Result<ImportSummary, NotStarted> {
    let profile = arguments.profile.as_deref().map(load_profile).transpose()?;
    let state_folder = process_state_folder()?;
    import_with(
        arguments,
        profile,
        Surroundings {
            state_folder: &state_folder,
            free_space: &StatvfsFreeSpace::new(),
            output: &mut io::stdout().lock(),
            errors: &mut io::stderr(),
        },
    )
}

/// The settings of the import, each from the command line, else the profile, else the
/// default of CLI-001 (CLI-010, "Settings").
struct ChosenSettings {
    destination: PathBuf,
    template: NameTemplate,
    job_code: Option<JobCode>,
    policy: ConflictPolicy,
    /// As given on the command line, or the profile's paths with `~` replaced.
    backups: Vec<PathBuf>,
    /// The video destination, if the command line or the profile names one (CLI-012).
    video_destination: Option<PathBuf>,
    /// The video backup destinations; empty means the backup destinations (CLI-012).
    video_backups: Vec<PathBuf>,
}

/// Takes each setting from the command line, else from `profile`, else the default. An
/// option replaces the profile's value, lists included (ADR 0032, decision 5).
///
/// # Errors
///
/// Returns the message for a missing destination, an invalid template or job code, or
/// a template with `{job}` and no job code.
fn choose_settings(
    arguments: &ImportArguments,
    profile: Option<ImportProfile>,
) -> Result<ChosenSettings, String> {
    let video_destination = arguments.video_destination.clone().or_else(|| {
        profile
            .as_ref()
            .and_then(|profile| profile.video_destination.clone())
    });
    let video_backups = match &profile {
        _ if !arguments.video_backups.is_empty() => arguments.video_backups.clone(),
        Some(profile) => profile.video_backup_destinations.clone(),
        None => Vec::new(),
    };
    let (profile_destination, profile_template, profile_policy, profile_job_code, profile_backups) =
        match profile {
            Some(profile) => (
                profile.destination,
                Some(profile.template),
                Some(profile.on_conflict),
                profile.job_codes.into_iter().next(),
                profile.backup_destinations,
            ),
            None => (None, None, None, None, Vec::new()),
        };
    let destination = arguments
        .destination
        .clone()
        .or(profile_destination)
        .ok_or_else(|| missing_destination_message(arguments.profile.as_deref()))?;
    let template = match (&arguments.template, profile_template) {
        (Some(text), _) => NameTemplate::parse(text)
            .map_err(|error| format!("invalid name template {text}: {error}"))?,
        (None, Some(template)) => template,
        (None, None) => NameTemplate::parse(DEFAULT_NAME_TEMPLATE)
            .map_err(|error| format!("invalid name template {DEFAULT_NAME_TEMPLATE}: {error}"))?,
    };
    let job_code = job_code(arguments.job_code.as_deref(), profile_job_code, &template)?;
    let policy = arguments
        .on_conflict
        .map(conflict_policy)
        .or(profile_policy)
        .unwrap_or(ConflictPolicy::AddSuffix);
    let backups = if arguments.backups.is_empty() {
        profile_backups
    } else {
        arguments.backups.clone()
    };
    Ok(ChosenSettings {
        destination,
        template,
        job_code,
        policy,
        backups,
        video_destination,
        video_backups,
    })
}

/// The message when neither the command line nor the profile names a destination;
/// without a profile, clap has already required one.
fn missing_destination_message(profile: Option<&str>) -> String {
    format!(
        "no destination: give one, or set destination in the profile {}",
        profile.unwrap_or_default()
    )
}

/// Checks the name of `--profile`, finds the configuration folder and reads the
/// profile (CLI-010, "Finding the profile"), before anything is scanned.
///
/// # Errors
///
/// Returns the message of CLI-010 for each way this fails.
fn load_profile(name: &str) -> Result<ImportProfile, String> {
    check_profile_name(name)?;
    let folder = process_configuration_folder()?;
    let home = process_home_folder()?;
    read_profile(
        &StdFileSystem::new(),
        &SaphyrParser::new(),
        &profiles_folder(&folder),
        name,
        &home,
    )
}

/// The state folder of CLI-004, CLI-008 and CLI-009, which holds the persisted sequence
/// counters (ADR 0029) and the import history (ADR 0031).
///
/// # Errors
///
/// Returns the message when the state folder is unknown: `XDG_STATE_HOME` is not an
/// absolute path and the home folder is unknown.
pub(crate) fn process_state_folder() -> Result<PathBuf, String> {
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
/// warns when the template does not use it. Without `--job-code`, the profile's first
/// job code, already checked against IMPORT-006, is used without a warning (CLI-010).
///
/// # Errors
///
/// Returns the message for an invalid job code, or for a template with `{job}` and no
/// job code.
fn job_code(
    text: Option<&str>,
    profile_job_code: Option<JobCode>,
    template: &NameTemplate,
) -> Result<Option<JobCode>, String> {
    let given = text
        .map(|text| {
            JobCode::parse(text).map_err(|error| format!("invalid job code {text:?}: {error}"))
        })
        .transpose()?;
    if given.is_some() && !template.uses_job_code() {
        report_warning("--job-code is not used, because the name template has no {job}");
    }
    let job_code = given.or(profile_job_code);
    require_job_code(template, job_code.as_ref())
        .map_err(|error| format!("{error}; give one with --job-code <text>"))?;
    Ok(job_code)
}

fn conflict_policy(on_conflict: OnConflict) -> ConflictPolicy {
    match on_conflict {
        OnConflict::AddSuffix => ConflictPolicy::AddSuffix,
        OnConflict::Skip => ConflictPolicy::Skip,
        OnConflict::Fail => ConflictPolicy::Fail,
    }
}
