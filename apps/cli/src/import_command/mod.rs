// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` (CLI-001, CLI-010, CLI-013, CLI-015): wires the adapters to the
//! import job of each source.

#[cfg(test)]
mod camera_tests;
mod folders;
mod locations;
mod run;
mod sources;
#[cfg(test)]
mod tests;

use std::io;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use adapter_free_space::StatvfsFreeSpace;
use adapter_fs::StdFileSystem;
use adapter_gphoto2::Gphoto2Cameras;
use adapter_yaml::SaphyrParser;
use import::{
    ConflictPolicy, DEFAULT_NAME_TEMPLATE, ImportProfile, ImportSummary, JobCode, require_job_code,
    require_video_job_code,
};
use naming::NameTemplate;

use crate::arguments::{ImportArguments, OnConflict};
use crate::guided::state_folder;
use crate::profiles::{
    check_profile_name, process_configuration_folder, process_home_folder, profiles_folder,
    read_profile,
};
use crate::report::has_backup_failures;
use crate::{FILES_FAILED, NOT_STARTED, report_error, report_hint, report_warning};
use locations::{Locations, locations};
use run::{NotStarted, SourceResults, Surroundings, import_with};

/// Runs the import of each source and returns the highest of their exit codes
/// (CLI-001, CLI-015).
pub(crate) fn run_import(arguments: &ImportArguments) -> ExitCode {
    let results = match import(arguments) {
        Ok(results) => results,
        Err(not_started) => {
            report_not_started(&not_started);
            return ExitCode::from(NOT_STARTED);
        }
    };
    let worst = results.iter().map(exit_code).max().unwrap_or_default();
    ExitCode::from(worst)
}

/// The exit code of one source (CLI-001); its errors are already reported.
fn exit_code(result: &Result<ImportSummary, NotStarted>) -> u8 {
    match result {
        Ok(summary) if summary.has_failures() || has_backup_failures(summary) => FILES_FAILED,
        Ok(_) => 0,
        Err(_) => NOT_STARTED,
    }
}

/// Reports each error that kept every source from starting, and then its hint.
fn report_not_started(not_started: &NotStarted) {
    for message in &not_started.messages {
        report_error(message);
    }
    if let Some(hint) = &not_started.hint {
        report_hint(hint);
    }
}

/// Splits the arguments into sources and destination, reads the profile and the state
/// folder from the process environment, in this order (CLI-016), and imports with the
/// real free space, standard output and standard error.
///
/// # Errors
///
/// Returns the messages of the errors that kept every source from starting.
fn import(arguments: &ImportArguments) -> Result<SourceResults, NotStarted> {
    let locations = locations(arguments)?;
    let profile = arguments.profile.as_deref().map(load_profile).transpose()?;
    let state_folder = process_state_folder()?;
    import_with(
        arguments,
        &locations,
        profile,
        Surroundings {
            state_folder: &state_folder,
            free_space: Arc::new(StatvfsFreeSpace::new()),
            cameras: &Gphoto2Cameras::new(),
            output: &mut io::stdout(),
            errors: &mut io::stderr(),
        },
    )
}

/// The settings of the import, each from the command line, else the profile, else the
/// default of CLI-001 (CLI-010, "Settings").
struct ChosenSettings {
    destination: PathBuf,
    template: NameTemplate,
    /// The video name template, if the command line or the profile names one (CLI-012).
    video_template: Option<NameTemplate>,
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
/// option replaces the profile's value, lists included (ADR 0032, decision 5). The
/// destination always comes from the command line (CLI-016).
///
/// # Errors
///
/// Returns the message for an invalid template or job code, or a template with `{job}`
/// and no job code.
fn choose_settings(
    arguments: &ImportArguments,
    locations: &Locations,
    profile: Option<ImportProfile>,
) -> Result<ChosenSettings, NotStarted> {
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
    let video_template = video_template(arguments, profile.as_ref())?;
    let (profile_template, profile_policy, profile_job_code, profile_backups) = match profile {
        Some(profile) => (
            Some(profile.template),
            Some(profile.on_conflict),
            profile.job_codes.into_iter().next(),
            profile.backup_destinations,
        ),
        None => (None, None, None, Vec::new()),
    };
    let destination = locations.destination.clone();
    let template = match (&arguments.template, profile_template) {
        (Some(text), _) => NameTemplate::parse(text)
            .map_err(|error| format!("invalid name template {text}: {error}"))?,
        (None, Some(template)) => template,
        (None, None) => NameTemplate::parse(DEFAULT_NAME_TEMPLATE)
            .map_err(|error| format!("invalid name template {DEFAULT_NAME_TEMPLATE}: {error}"))?,
    };
    let job_code = job_code(
        arguments.job_code.as_deref(),
        profile_job_code,
        &template,
        video_template.as_ref(),
    )?;
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
        video_template,
        job_code,
        policy,
        backups,
        video_destination,
        video_backups,
    })
}

/// The video name template from `--video-template`, else from `profile`, else none
/// (CLI-012).
///
/// # Errors
///
/// Returns the message for an invalid `--video-template`.
fn video_template(
    arguments: &ImportArguments,
    profile: Option<&ImportProfile>,
) -> Result<Option<NameTemplate>, String> {
    match &arguments.video_template {
        Some(text) => NameTemplate::parse(text)
            .map(Some)
            .map_err(|error| format!("invalid video name template {text}: {error}")),
        None => Ok(profile.and_then(|profile| profile.video_template.clone())),
    }
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
    let profiles = profiles_folder(&folder);
    read_profile(
        &StdFileSystem::new(),
        &SaphyrParser::new(),
        &profiles,
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

/// Checks `--job-code` against IMPORT-006 and the templates (CLI-001, "Job code"), and
/// warns when no template uses it. Without `--job-code`, the profile's first
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
    video_template: Option<&NameTemplate>,
) -> Result<Option<JobCode>, String> {
    let given = text
        .map(|text| {
            JobCode::parse(text).map_err(|error| format!("invalid job code {text:?}: {error}"))
        })
        .transpose()?;
    let video_uses_job_code = video_template.is_some_and(NameTemplate::uses_job_code);
    if given.is_some() && !template.uses_job_code() && !video_uses_job_code {
        report_warning("--job-code is not used, because the name template has no {job}");
    }
    let job_code = given.or(profile_job_code);
    require_job_code(template, job_code.as_ref())
        .and_then(|()| {
            video_template.map_or(Ok(()), |video_template| {
                require_video_job_code(video_template, job_code.as_ref())
            })
        })
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
