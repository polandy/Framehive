// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` (CLI-001, CLI-010, CLI-013, CLI-015): wires the adapters to the
//! import job of each source.

#[cfg(test)]
mod camera_tests;
mod folders;
mod run;
mod sources;
#[cfg(test)]
mod tests;

use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use adapter_free_space::StatvfsFreeSpace;
use adapter_fs::StdFileSystem;
use adapter_gphoto2::Gphoto2Cameras;
use adapter_yaml::SaphyrParser;
use import::{
    ConflictPolicy, DEFAULT_NAME_TEMPLATE, ImportProfile, ImportSummary, JobCode, require_job_code,
};
use naming::NameTemplate;

use crate::arguments::{ImportArguments, OnConflict};
use crate::cameras::camera_choice;
use crate::guided::state_folder;
use crate::profiles::{
    check_profile_name, process_configuration_folder, process_home_folder, profile_path,
    profiles_folder, read_profile,
};
use crate::report::has_backup_failures;
use crate::{FILES_FAILED, NOT_STARTED, report_error, report_hint, report_warning};
use run::{NotStarted, SourceResults, Surroundings, import_with};

/// Runs the import of each source and returns the highest of their exit codes
/// (CLI-001, CLI-015).
pub(crate) fn run_import(arguments: &ImportArguments) -> ExitCode {
    let results = match import(arguments) {
        Ok(results) => results,
        Err(not_started) => {
            report_not_started(&not_started, None);
            return ExitCode::from(NOT_STARTED);
        }
    };
    let several = arguments.sources.len() > 1;
    let worst = arguments
        .sources
        .iter()
        .zip(&results)
        .map(|(source, result)| exit_code(result, several.then_some(source.as_path())))
        .max()
        .unwrap_or_default();
    ExitCode::from(worst)
}

/// The exit code of one source (CLI-001), after reporting each error that kept it from
/// starting, after `named_source` when there are several sources (CLI-015).
fn exit_code(result: &Result<ImportSummary, NotStarted>, named_source: Option<&Path>) -> u8 {
    match result {
        Ok(summary) if summary.has_failures() || has_backup_failures(summary) => FILES_FAILED,
        Ok(_) => 0,
        Err(not_started) => {
            report_not_started(not_started, named_source);
            NOT_STARTED
        }
    }
}

/// Reports each error that kept the import from starting, after `named_source` when
/// there are several sources (CLI-015), and then its hint.
fn report_not_started(not_started: &NotStarted, named_source: Option<&Path>) {
    for message in &not_started.messages {
        match named_source {
            Some(source) => report_error(&format!("{}: {message}", source.display())),
            None => report_error(message),
        }
    }
    if let Some(hint) = &not_started.hint {
        report_hint(hint);
    }
}

/// Reads the profile and the state folder from the process environment, and imports
/// with the real free space, standard output and standard error.
///
/// # Errors
///
/// Returns the messages of the errors that kept every source from starting.
fn import(arguments: &ImportArguments) -> Result<SourceResults, NotStarted> {
    require_source(arguments)?;
    let profile = arguments.profile.as_deref().map(load_profile).transpose()?;
    let state_folder = process_state_folder()?;
    import_with(
        arguments,
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

/// Checks that the command names a source (CLI-001), before the profile is read.
///
/// # Errors
///
/// Returns the message and an example when no source is given.
fn require_source(arguments: &ImportArguments) -> Result<(), NotStarted> {
    if arguments.sources.is_empty() {
        return Err(NotStarted::with_hint(
            "no source: give the folders or cameras to import from".to_owned(),
            "framehive import /media/card --destination ~/Pictures".to_owned(),
        ));
    }
    Ok(())
}

/// An import profile and the file it was read from (CLI-010).
pub(super) struct LoadedProfile {
    pub(super) profile: ImportProfile,
    pub(super) path: PathBuf,
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
/// Returns the message for a missing destination, with a hint when the command looks
/// like one that named the destination by position, an invalid template or job code,
/// or a template with `{job}` and no job code.
fn choose_settings(
    arguments: &ImportArguments,
    loaded: Option<LoadedProfile>,
) -> Result<ChosenSettings, NotStarted> {
    let profile_path = loaded.as_ref().map(|loaded| loaded.path.clone());
    let profile = loaded.map(|loaded| loaded.profile);
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
        .ok_or_else(|| missing_destination(arguments, profile_path.as_deref()))?;
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

/// The error when neither `--destination` nor the profile names a destination
/// (CLI-016), with a hint when two or more sources end with a folder: the
/// command of someone who still names the destination by position. The hint depends
/// on the arguments alone.
fn missing_destination(arguments: &ImportArguments, profile_path: Option<&Path>) -> NotStarted {
    let base = "no destination: give --destination <folder>";
    let message = match (&arguments.profile, profile_path) {
        (Some(name), Some(path)) => format!(
            "{base}, or set destination in the profile {name}: {}",
            path.display()
        ),
        _ => base.to_owned(),
    };
    match positional_destination_hint(&arguments.sources) {
        Some(hint) => NotStarted::with_hint(message, hint),
        None => NotStarted::from(message),
    }
}

/// `every folder is a source; to import into <last>, write: framehive import <others>
/// --destination <last>`, when there are two or more sources and the last is a folder.
fn positional_destination_hint(sources: &[PathBuf]) -> Option<String> {
    let (last, others) = sources.split_last()?;
    if others.is_empty() || camera_choice(last).is_some() {
        return None;
    }
    let others = others
        .iter()
        .map(|source| source.display().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let last = last.display();
    Some(format!(
        "every folder is a source; to import into {last}, write: framehive import {others} --destination {last}"
    ))
}

/// Checks the name of `--profile`, finds the configuration folder and reads the
/// profile (CLI-010, "Finding the profile"), before anything is scanned.
///
/// # Errors
///
/// Returns the message of CLI-010 for each way this fails.
fn load_profile(name: &str) -> Result<LoadedProfile, String> {
    check_profile_name(name)?;
    let folder = process_configuration_folder()?;
    let home = process_home_folder()?;
    let profiles = profiles_folder(&folder);
    let profile = read_profile(
        &StdFileSystem::new(),
        &SaphyrParser::new(),
        &profiles,
        name,
        &home,
    )?;
    Ok(LoadedProfile {
        profile,
        path: profile_path(&profiles, name),
    })
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
