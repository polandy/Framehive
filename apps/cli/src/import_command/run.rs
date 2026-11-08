// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import of `framehive import` once its settings are chosen (CLI-001, CLI-005,
//! CLI-011). It takes the state folder, the free space and the output from its caller,
//! so that tests can give their own.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use adapter_blake3::Blake3Hasher;
use adapter_clock::SystemClock;
use adapter_fs::StdFileSystem;
use adapter_sequence_counters::FileSequenceCounters;
use adapter_sqlite::SqliteImportHistory;
use files::{Cameras, FreeSpace};
use import::{
    CAMERA_FORMAT_READINESS_NOTE, DryRunMemory, FreeSpaceUnknown, ImportJob, ImportJobSettings,
    ImportPorts, ImportProfile, ImportRun, ImportSettings, ImportSummary, PlannedRun, ProgressSink,
    ReservedTargetNames, RunError, SessionSequence, SpaceShortage, scan_source,
};
use jobs::JobQueue;
use naming::CompatibilityMode;

use super::folders::{Folders, backup_names, resolve_sources};
use super::locations::Locations;
use super::sources::{Enqueued, Streams};
use super::{ChosenSettings, choose_settings};
use crate::arguments::ImportArguments;
use crate::cameras::{OpenedCamera, camera_choice, open_camera};
use crate::guided::space_text;
use crate::report::ReportPrinter;
use crate::{JOB_ENDED_UNEXPECTEDLY, write_warning};

/// What the import takes from the process: the tests give their own.
pub(super) struct Surroundings<'a> {
    /// The state folder of CLI-004, with the sequence counters and the import history.
    pub(super) state_folder: &'a Path,
    /// The free space of the destinations (IMPORT-012).
    pub(super) free_space: Arc<dyn FreeSpace + Send + Sync>,
    /// The connected cameras, for a `camera:` source (CLI-013).
    pub(super) cameras: &'a dyn Cameras,
    /// Standard output, for the report.
    pub(super) output: &'a mut (dyn Write + Send),
    /// Standard error, for the warnings and the errors of a run that started.
    pub(super) errors: &'a mut (dyn Write + Send),
}

/// The import did not start: the messages for standard error, one per line, without
/// `error: `, and a hint that shows what to write instead, without `hint: `.
#[derive(Debug)]
pub(super) struct NotStarted {
    pub(super) messages: Vec<String>,
    pub(super) hint: Option<String>,
}

impl NotStarted {
    /// One message and the hint that follows it.
    pub(super) fn with_hint(message: String, hint: String) -> Self {
        NotStarted {
            messages: vec![message],
            hint: Some(hint),
        }
    }
}

impl From<String> for NotStarted {
    fn from(message: String) -> Self {
        NotStarted {
            messages: vec![message],
            hint: None,
        }
    }
}

/// For a `camera:` source, the opened camera. It is opened only once the folders of
/// every source are known to be fine.
///
/// # Errors
///
/// Returns the message when the camera cannot be opened.
fn open_source_camera(
    source: &Path,
    cameras: &dyn Cameras,
) -> Result<Option<OpenedCamera>, NotStarted> {
    let camera = camera_choice(source)
        .map(|choice| open_camera(cameras, &choice))
        .transpose()?;
    Ok(camera)
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
        video_template: chosen.video_template.as_ref(),
        job_code: chosen.job_code.as_ref(),
        mode: CompatibilityMode::On,
        policy: chosen.policy,
        dry_run,
    }
}

/// The arguments, one source as given, the settings chosen for every source, and the
/// checked folders of that source.
type Given<'a> = (
    &'a ImportArguments,
    &'a Path,
    &'a ChosenSettings,
    &'a Folders,
);

/// What became of each source, in the order given (CLI-015).
pub(super) type SourceResults = Vec<Result<ImportSummary, NotStarted>>;

/// Chooses the settings and checks every source (CLI-016), then plans and enqueues the
/// job of each source in the order given, and prints each job's report, and the errors
/// of a source that cannot start, as they come (CLI-001, CLI-015).
///
/// # Errors
///
/// Returns the messages of the errors that kept every source from starting; they are
/// not printed yet.
pub(super) fn import_with(
    arguments: &ImportArguments,
    locations: &Locations,
    profile: Option<ImportProfile>,
    surroundings: Surroundings<'_>,
) -> Result<SourceResults, NotStarted> {
    let chosen = choose_settings(arguments, locations, profile)?;
    let sources = &locations.sources;
    let checked = resolve_sources(sources, &chosen)?;
    let mut ports = import_ports(surroundings.state_folder, surroundings.free_space);
    // A dry run counts the numbers and target names across the sources in memory, so
    // it shows what a real run would do (IMPORT-016).
    let dry_run_memory = arguments.dry_run.then(|| {
        let memory = Arc::new(DryRunMemory::new(Arc::clone(&ports.counters)));
        ports.counters = Arc::clone(&memory) as _;
        memory
    });
    let streams = Streams::new(surroundings.output, surroundings.errors);
    let streams = &streams;
    let several = sources.len() > 1;
    let (mut queue, _updates) = JobQueue::start();
    // Each opened camera is released once every job ended (CLI-013).
    let mut opened = Vec::new();
    let results = thread::scope(|scope| {
        let following: Vec<_> = sources
            .iter()
            .zip(&checked)
            .map(|(source, folders)| {
                let given = (arguments, source.as_path(), &chosen, folders);
                let named = several.then_some(source.as_path());
                // A source that cannot start is reported at once; the others go on.
                let (enqueued, camera) = streams.reported(
                    enqueue_source(
                        given,
                        (&ports, dry_run_memory.as_ref()),
                        surroundings.cameras,
                        &mut queue,
                        streams,
                    ),
                    named,
                )?;
                opened.extend(camera);
                Ok(scope.spawn(move || streams.reported(enqueued.follow(named, streams), named)))
            })
            .collect();
        following
            .into_iter()
            .zip(sources)
            .map(|(following, source)| {
                following.and_then(|thread| {
                    thread.join().unwrap_or_else(|_| {
                        let ended = Err(NotStarted::from(JOB_ENDED_UNEXPECTEDLY.to_owned()));
                        streams.reported(ended, several.then_some(source.as_path()))
                    })
                })
            })
            .collect()
    });
    Ok(results)
}

/// Opens the camera of one source, checks its free space, prepares its destinations,
/// plans its import and enqueues its job, in a dry run with the dry-run memory.
///
/// # Errors
///
/// Returns the messages of the errors that kept the source from starting.
fn enqueue_source<'f>(
    (arguments, source, chosen, folders): Given<'f>,
    (ports, dry_run_memory): (&ImportPorts, Option<&Arc<DryRunMemory>>),
    cameras: &dyn Cameras,
    queue: &mut JobQueue,
    streams: &Streams<'_>,
) -> Result<(Enqueued<'f>, Option<OpenedCamera>), NotStarted> {
    let camera = open_source_camera(source, cameras)?;
    let backup_paths = folders.backup_paths();
    let video_backup_paths = folders.video_backup_paths();
    let settings = import_settings(
        folders,
        chosen,
        (&backup_paths, &video_backup_paths),
        arguments.dry_run,
    );
    let mut run = ports.import_run();
    let mut printer =
        ReportPrinter::new(Vec::new(), arguments.dry_run).with_backup_names(backup_names(folders));
    let scan = if let Some(camera) = &camera {
        run = run
            .with_source(&camera.file_system)
            .with_source_name(&camera.name);
        printer = printer.with_format_readiness_note(CAMERA_FORMAT_READINESS_NOTE);
        camera.scan(&folders.source)
    } else {
        scan_source(
            &*ports.file_system,
            &folders.source,
            &folders.destination.path,
        )
        .map_err(|error| error.to_string())
    }?;
    printer.scan_finished(&scan);
    streams.print(&printer.take_written());
    let plan = run
        .plan(&settings, &scan)
        .map_err(|error| error.to_string())?;
    let space = check_free_space(&run, &settings, folders, &plan, &mut **streams.errors());
    let warned = match space {
        Ok(warned) => warned,
        Err(shortages) => {
            if arguments.dry_run {
                // A dry run still lists what it would do (CLI-011.3).
                run.execute(&settings, plan, &mut printer);
                streams.print(&printer.take_written());
            }
            return Err(shortages);
        }
    };
    let (job, reports) = ImportJob::new(ports.clone(), ImportJobSettings::of(&settings));
    let mut job = job.with_plan(plan);
    if let Some(memory) = dry_run_memory {
        job = job.with_dry_run_memory(Arc::clone(memory));
    }
    if let Some(camera) = &camera {
        job = job.with_source(Box::new(camera.file_system.clone()), camera.name.clone());
    }
    job.enqueue(queue)
        .map_err(|error| start_error(error, folders))?;
    let enqueued = Enqueued {
        reports,
        printer,
        folders,
        warned,
    };
    Ok((enqueued, camera))
}

/// The ports of the import: the library on this machine, the state in `state_folder`.
fn import_ports(state_folder: &Path, free_space: Arc<dyn FreeSpace + Send + Sync>) -> ImportPorts {
    ImportPorts {
        file_system: Arc::new(StdFileSystem::new()),
        hasher: Arc::new(Blake3Hasher),
        clock: Arc::new(SystemClock::new()),
        counters: Arc::new(FileSequenceCounters::new(state_folder.to_path_buf())),
        session: Arc::new(SessionSequence::new()),
        history: Arc::new(SqliteImportHistory::new(state_folder)),
        free_space,
        reserved_names: Arc::new(ReservedTargetNames::new()),
    }
}

/// Checks that each file system can hold what the run writes to it, before any folder
/// is created, and warns about each destination whose free space is unknown (CLI-011);
/// returns those destinations.
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
) -> Result<Vec<PathBuf>, NotStarted> {
    let check = run.check_free_space(settings, plan);
    for unknown in &check.unknown {
        write_warning(errors, &unknown_free_space_message(unknown, folders));
    }
    if check.shortages.is_empty() {
        return Ok(check
            .unknown
            .into_iter()
            .map(|unknown| unknown.destination)
            .collect());
    }
    Err(NotStarted {
        messages: check
            .shortages
            .iter()
            .map(|shortage| shortage_message(shortage, folders))
            .collect(),
        hint: None,
    })
}

/// `cannot tell the free space at <folder as given>: <reason>` (CLI-011).
pub(super) fn unknown_free_space_message(unknown: &FreeSpaceUnknown, folders: &Folders) -> String {
    format!(
        "cannot tell the free space at {}: {}",
        folders.shown(&unknown.destination).display(),
        unknown.error
    )
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

/// The messages for the run error of a job that did not start, naming the folders as
/// given (CLI-001, CLI-011, CLI-012). The job creates the missing destinations only
/// after its own free-space check (IMPORT-012).
pub(super) fn start_error(error: RunError, folders: &Folders) -> NotStarted {
    match error {
        RunError::NotEnoughSpace(not_enough) => NotStarted {
            messages: not_enough
                .shortages
                .iter()
                .map(|shortage| shortage_message(shortage, folders))
                .collect(),
            hint: None,
        },
        RunError::DestinationNotCreated(not_created) => {
            NotStarted::from(folders.not_created_message(not_created))
        }
        error => NotStarted::from(error.to_string()),
    }
}
