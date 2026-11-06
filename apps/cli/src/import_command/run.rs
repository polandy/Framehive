// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import of `framehive import` once its settings are chosen (CLI-001, CLI-005,
//! CLI-011). It takes the state folder, the free space and the output from its caller,
//! so that tests can give their own.

use std::io::Write;
use std::path::{Path, PathBuf};

use adapter_blake3::Blake3Hasher;
use adapter_clock::SystemClock;
use adapter_fs::StdFileSystem;
use adapter_sequence_counters::FileSequenceCounters;
use adapter_sqlite::SqliteImportHistory;
use files::{Cameras, FileSystem, FreeSpace};
use import::{
    CAMERA_FORMAT_READINESS_NOTE, ImportProfile, ImportRun, ImportSettings, ImportSummary,
    PlannedRun, ProgressSink, SessionSequence, SpaceShortage, scan_source,
};
use naming::CompatibilityMode;

use super::folders::{Folders, backup_names, resolve_folders, unusable_message};
use super::{ChosenSettings, choose_settings};
use crate::arguments::ImportArguments;
use crate::cameras::{OpenedCamera, camera_choice, open_camera};
use crate::guided::space_text;
use crate::paths::create_destination;
use crate::report::ReportPrinter;
use crate::{write_error, write_warning};

/// What the import takes from the process: the tests give their own.
pub(super) struct Surroundings<'a> {
    /// The state folder of CLI-004, with the sequence counters and the import history.
    pub(super) state_folder: &'a Path,
    /// The free space of the destinations (IMPORT-012).
    pub(super) free_space: &'a dyn FreeSpace,
    /// The connected cameras, for a `camera:` source (CLI-013).
    pub(super) cameras: &'a dyn Cameras,
    /// Standard output, for the report.
    pub(super) output: &'a mut dyn Write,
    /// Standard error, for the warnings and the errors of a run that started.
    pub(super) errors: &'a mut dyn Write,
}

/// The import did not start: the messages for standard error, one per line, without
/// `error: `.
#[derive(Debug)]
pub(super) struct NotStarted(pub(super) Vec<String>);

impl From<String> for NotStarted {
    fn from(message: String) -> Self {
        NotStarted(vec![message])
    }
}

/// The checked folders and, for a `camera:` source, the opened camera. The camera is
/// opened only once the folders are known to be fine.
///
/// # Errors
///
/// Returns the messages for unusable folders or a camera that cannot be opened.
fn folders_and_camera(
    arguments: &ImportArguments,
    chosen: &ChosenSettings,
    cameras: &dyn Cameras,
) -> Result<(Folders, Option<OpenedCamera>), NotStarted> {
    let choice = camera_choice(&arguments.source);
    let folders = resolve_folders(arguments, chosen, choice.is_some())?;
    let camera = choice
        .map(|choice| open_camera(cameras, &choice))
        .transpose()?;
    Ok((folders, camera))
}

/// The settings of the run from the checked `folders`, the `chosen` settings and the
/// paths of the backup and video backup destinations.
fn import_settings<'a>(
    folders: &'a Folders,
    chosen: &'a ChosenSettings,
    (backup_paths, video_backup_paths): (&'a [PathBuf], &'a [PathBuf]),
    dry_run: bool,
) -> ImportSettings<'a> {
    ImportSettings {
        source: &folders.source,
        destination: &folders.destination.path,
        backup_destinations: backup_paths,
        video_destination: folders.video_destination_path(),
        video_backup_destinations: video_backup_paths,
        template: &chosen.template,
        job_code: chosen.job_code.as_ref(),
        mode: CompatibilityMode::On,
        policy: chosen.policy,
        dry_run,
    }
}

/// Checks the arguments, checks the free space, prepares the destinations and runs the
/// import, printing the report as it goes.
///
/// # Errors
///
/// Returns the messages of the errors that kept the import from starting.
pub(super) fn import_with(
    arguments: &ImportArguments,
    profile: Option<ImportProfile>,
    surroundings: Surroundings<'_>,
) -> Result<ImportSummary, NotStarted> {
    let chosen = choose_settings(arguments, profile)?;
    // The camera is released when this function returns (CLI-013).
    let (folders, camera) = folders_and_camera(arguments, &chosen, surroundings.cameras)?;
    let backup_paths = folders.backup_paths();
    let video_backup_paths = folders.video_backup_paths();
    let settings = import_settings(
        &folders,
        &chosen,
        (&backup_paths, &video_backup_paths),
        arguments.dry_run,
    );
    let file_system = StdFileSystem::new();
    let clock = SystemClock::new();
    let counters = FileSequenceCounters::new(surroundings.state_folder.to_path_buf());
    let session = SessionSequence::new();
    let history = SqliteImportHistory::new(surroundings.state_folder);
    let mut run = ImportRun::new(
        &file_system,
        &Blake3Hasher,
        &clock,
        &counters,
        &session,
        &history,
        surroundings.free_space,
    );
    let mut printer = ReportPrinter::new(surroundings.output, arguments.dry_run)
        .with_backup_names(backup_names(&folders));
    let scan = if let Some(camera) = &camera {
        run = run
            .with_source(camera.file_system.as_ref())
            .with_source_name(&camera.name);
        printer = printer.with_format_readiness_note(CAMERA_FORMAT_READINESS_NOTE);
        camera.scan(&folders.source)
    } else {
        scan_source(&file_system, &folders.source, &folders.destination.path)
            .map_err(|error| error.to_string())
    }?;
    printer.scan_finished(&scan);
    let plan = run
        .plan(&settings, &scan)
        .map_err(|error| error.to_string())?;
    if let Err(shortages) = check_free_space(&run, &settings, &folders, &plan, surroundings.errors)
    {
        if arguments.dry_run {
            // A dry run still lists what it would do (CLI-011.3).
            run.execute(&settings, plan, &mut printer);
        }
        return Err(shortages);
    }
    if !arguments.dry_run {
        create_destinations(&file_system, &folders)?;
    }
    let plan = run
        .start(&settings, plan)
        .map_err(|error| error.to_string())?;
    let summary = run.execute(&settings, plan, &mut printer);
    printer.print_summary(&summary);
    if let Some(error) = &summary.history_error {
        write_error(surroundings.errors, &error.to_string());
    }
    Ok(summary)
}

/// Checks that each file system can hold what the run writes to it, before any folder
/// is created, and warns about each destination whose free space is unknown (CLI-011).
///
/// # Errors
///
/// Returns one message per file system that is short.
fn check_free_space(
    run: &ImportRun<'_>,
    settings: &ImportSettings<'_>,
    folders: &Folders,
    plan: &PlannedRun,
    errors: &mut dyn Write,
) -> Result<(), NotStarted> {
    let check = run.check_free_space(settings, plan);
    for unknown in &check.unknown {
        let message = format!(
            "cannot tell the free space at {}: {}",
            folders.shown(&unknown.destination).display(),
            unknown.error
        );
        write_warning(errors, &message);
    }
    if check.shortages.is_empty() {
        return Ok(());
    }
    Err(NotStarted(
        check
            .shortages
            .iter()
            .map(|shortage| shortage_message(shortage, folders))
            .collect(),
    ))
}

/// `not enough space at <folders as given>: needs 12.8 GB, 3.1 GB free` (CLI-011).
fn shortage_message(shortage: &SpaceShortage, folders: &Folders) -> String {
    let shown = shortage
        .destinations
        .iter()
        .map(|destination| folders.shown(destination).display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("not enough space at {shown}: {}", space_text(shortage))
}

/// Creates the destination, the video destination and each backup destination and
/// video backup destination. A folder that is two of them is created once, and the
/// second attempt finds it there.
///
/// # Errors
///
/// Returns the message for the first folder that cannot be created.
fn create_destinations(file_system: &dyn FileSystem, folders: &Folders) -> Result<(), String> {
    create_destination(file_system, &folders.destination).map_err(|error| error.to_string())?;
    let named = folders
        .video_destination
        .iter()
        .map(|folder| ("video destination", folder))
        .chain(
            folders
                .backups
                .iter()
                .map(|folder| ("backup destination", folder)),
        )
        .chain(
            folders
                .video_backups
                .iter()
                .map(|folder| ("video backup destination", folder)),
        );
    for (kind, (folder, given)) in named {
        create_destination(file_system, folder)
            .map_err(|error| unusable_message(kind, given, &error))?;
    }
    Ok(())
}
