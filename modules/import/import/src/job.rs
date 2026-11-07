// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! An import as a job in the job queue (IMPORT-015, ADR 0035): it owns its settings,
//! its ports and its source, runs an [`ImportRun`] on the worker thread, and reports
//! to whoever enqueued it over a channel of its own.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use domain::Clock;
use files::{FileSystem, FreeSpace, Hasher, ReadOnlyFileSystem};
use history::ImportHistory;
use jobs::{Job, JobControl, JobEnd, ResourceKey};
use naming::{CompatibilityMode, NameTemplate, SequenceCounters};

use crate::conflicts::ConflictPolicy;
use crate::job_code::JobCode;
use crate::report::{FileReport, ImportSummary, ProgressSink, RunError};
use crate::reserved_names::{ReservedNames, ReservedTargetNames};
use crate::run::{ImportRun, ImportSettings, PlannedRun};
use crate::scan::SourceScan;
use crate::sequence::SessionSequence;

mod enqueue;
mod sending;

use sending::{Answer, SendingSteps};

/// The ports an import job works through, owned so they can move to the worker
/// thread and be shared with the application that enqueued it.
#[derive(Clone)]
pub struct ImportPorts {
    /// Reads the source, unless the job has a source of its own, and reads and writes
    /// the destinations.
    pub file_system: Arc<dyn FileSystem + Send + Sync>,
    /// The checksums of the verified copy.
    pub hasher: Arc<dyn Hasher + Send + Sync>,
    /// The import time.
    pub clock: Arc<dyn Clock + Send + Sync>,
    /// The persisted sequence counters (IMPORT-002).
    pub counters: Arc<dyn SequenceCounters + Send + Sync>,
    /// The session counter of this run of the application (IMPORT-002).
    pub session: Arc<SessionSequence>,
    /// The import history of this machine (IMPORT-009).
    pub history: Arc<dyn ImportHistory + Send + Sync>,
    /// How much the destinations can hold (IMPORT-012).
    pub free_space: Arc<dyn FreeSpace + Send + Sync>,
    /// The target names the import jobs of this application reserved (IMPORT-015).
    pub reserved_names: Arc<ReservedTargetNames>,
}

impl ImportPorts {
    /// An import run through these ports, on the caller's thread.
    #[must_use]
    pub fn import_run(&self) -> ImportRun<'_> {
        ImportRun::new(
            &*self.file_system,
            &*self.hasher,
            &*self.clock,
            &*self.counters,
            &self.session,
            &*self.history,
            &*self.free_space,
        )
        .with_reserved_names(&self.reserved_names, None)
    }
}

impl fmt::Debug for ImportPorts {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImportPorts")
            .finish_non_exhaustive()
    }
}

/// [`ImportSettings`] that the job owns, field by field.
#[derive(Debug, Clone)]
pub struct ImportJobSettings {
    /// See [`ImportSettings::source`].
    pub source: PathBuf,
    /// See [`ImportSettings::destination`].
    pub destination: PathBuf,
    /// See [`ImportSettings::backup_destinations`].
    pub backup_destinations: Vec<PathBuf>,
    /// See [`ImportSettings::video_destination`].
    pub video_destination: Option<PathBuf>,
    /// See [`ImportSettings::video_backup_destinations`].
    pub video_backup_destinations: Vec<PathBuf>,
    /// See [`ImportSettings::template`].
    pub template: NameTemplate,
    /// See [`ImportSettings::job_code`].
    pub job_code: Option<JobCode>,
    /// See [`ImportSettings::mode`].
    pub mode: CompatibilityMode,
    /// See [`ImportSettings::policy`].
    pub policy: ConflictPolicy,
    /// See [`ImportSettings::dry_run`].
    pub dry_run: bool,
}

impl ImportJobSettings {
    /// Owned copies of `settings`.
    #[must_use]
    pub fn of(settings: &ImportSettings<'_>) -> Self {
        ImportJobSettings {
            source: settings.source.to_path_buf(),
            destination: settings.destination.to_path_buf(),
            backup_destinations: settings.backup_destinations.to_vec(),
            video_destination: settings.video_destination.map(Path::to_path_buf),
            video_backup_destinations: settings.video_backup_destinations.to_vec(),
            template: settings.template.clone(),
            job_code: settings.job_code.cloned(),
            mode: settings.mode,
            policy: settings.policy,
            dry_run: settings.dry_run,
        }
    }

    /// These settings, borrowed as an import run takes them.
    #[must_use]
    pub fn settings(&self) -> ImportSettings<'_> {
        ImportSettings {
            source: &self.source,
            destination: &self.destination,
            backup_destinations: &self.backup_destinations,
            video_destination: self.video_destination.as_deref(),
            video_backup_destinations: &self.video_backup_destinations,
            template: &self.template,
            job_code: self.job_code.as_ref(),
            mode: self.mode,
            policy: self.policy,
            dry_run: self.dry_run,
        }
    }
}

/// A source on a file system of its own, such as an opened camera, that the job reads
/// and hands back when it ends (IMPORT-015.7).
pub type OwnSource = Box<dyn ReadOnlyFileSystem + Send>;

/// An import that runs as a job in the job queue (glossary: Import job).
///
/// It does what [`ImportRun::run`] does, or with [`ImportJob::with_plan`] what
/// [`ImportRun::start`] and [`ImportRun::execute`] do, and reports each step to the
/// [`ImportJobReports`] that [`ImportJob::new`] returns with it.
pub struct ImportJob {
    ports: ImportPorts,
    settings: ImportJobSettings,
    source: Option<(OwnSource, PathBuf)>,
    plan: Option<PlannedRun>,
    asks: bool,
    reports: Sender<Report>,
    /// The target names reserved when the job was enqueued, until it ends.
    reserved: Option<ReservedNames>,
}

/// What an import job sends to its enqueuer, in this order.
enum Report {
    ScanFinished(SourceScan),
    FileStarted(PathBuf),
    FileFinished(FileReport),
    /// Asks whether to go on with the next file.
    GoOn(Answer),
    Ended(ImportJobEnd),
}

impl ImportJob {
    /// A job that imports `settings.source` through `ports`, and the receiver of its
    /// reports.
    #[must_use]
    pub fn new(ports: ImportPorts, settings: ImportJobSettings) -> (Self, ImportJobReports) {
        let (reports, receiver) = mpsc::channel();
        let job = ImportJob {
            ports,
            settings,
            source: None,
            plan: None,
            asks: false,
            reports,
            reserved: None,
        };
        (job, ImportJobReports { receiver })
    }

    /// Reads the source through `file_system`, named `name` in the import history, as
    /// [`ImportRun::with_source`] and [`ImportRun::with_source_name`] do. The job hands
    /// it back when it ends.
    #[must_use]
    pub fn with_source(mut self, file_system: OwnSource, name: PathBuf) -> Self {
        self.source = Some((file_system, name));
        self
    }

    /// Imports `plan`, made by [`ImportRun::plan`] from the same settings, instead of
    /// scanning and planning the source again: the job starts and executes it
    /// (IMPORT-015.8). The scan is not reported then.
    #[must_use]
    pub fn with_plan(mut self, plan: PlannedRun) -> Self {
        self.plan = Some(plan);
        self
    }

    /// Asks the enqueuer before each file whether to go on, as
    /// [`ProgressSink::should_continue`] does for a run: the job waits for the answer
    /// that [`ImportJobReports::follow`] gives (IMPORT-015.9).
    #[must_use]
    pub fn asking_before_each_file(mut self) -> Self {
        self.asks = true;
        self
    }

    /// Runs the import and sends its end report; returns how it ended.
    fn import(self, control: &JobControl) -> JobEnd {
        let ImportJob {
            ports,
            settings,
            source,
            plan,
            asks,
            reports,
            reserved,
        } = self;
        let settings = settings.settings();
        let mut steps = SendingSteps::new(&reports, control, asks);
        let result = {
            let mut run = ports
                .import_run()
                .with_reserved_names(&ports.reserved_names, reserved.as_ref());
            if let Some((file_system, name)) = &source {
                run = run.with_source(&**file_system).with_source_name(name);
            }
            match plan {
                Some(plan) => run
                    .start(&settings, plan)
                    .map(|plan| run.execute_steps(&settings, plan, &mut steps)),
                None => run.run_steps(&settings, &mut steps),
            }
        };
        let end = match &result {
            Ok(summary) if summary.stopped => JobEnd::Stopped,
            Ok(_) => JobEnd::Finished,
            Err(error) => JobEnd::Failed(error.to_string()),
        };
        let source = source.map(|(file_system, _)| file_system);
        // Released before the enqueuer hears of the end, so a job it enqueues next
        // finds the names free.
        drop(reserved);
        let _ = reports.send(Report::Ended(ImportJobEnd { result, source }));
        end
    }
}

impl Job for ImportJob {
    fn resources(&self) -> Vec<ResourceKey> {
        self.resource_keys()
    }

    fn run(self: Box<Self>, control: &JobControl) -> JobEnd {
        self.import(control)
    }
}

/// How an import job ended, as its last report says.
pub struct ImportJobEnd {
    /// The import summary, or the run error when the import could not start.
    pub result: Result<ImportSummary, RunError>,
    /// The source the job was given with [`ImportJob::with_source`], handed back.
    pub source: Option<OwnSource>,
}

impl fmt::Debug for ImportJobEnd {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImportJobEnd")
            .field("result", &self.result)
            .field("source", &self.source.as_ref().map(|_| "own source"))
            .finish()
    }
}

/// The receiving end of an import job's reports.
#[derive(Debug)]
pub struct ImportJobReports {
    receiver: Receiver<Report>,
}

impl ImportJobReports {
    /// Shows each report of the job to `progress` as it comes, answers the job's
    /// questions whether to go on with [`ProgressSink::should_continue`], and returns
    /// the end report, whose summary holds every file report again.
    ///
    /// Returns `None` when the job ended without an end report: it was removed before
    /// it ran, or stopped unexpectedly.
    pub fn follow(self, progress: &mut dyn ProgressSink) -> Option<ImportJobEnd> {
        let mut files = Vec::new();
        for report in self.receiver {
            match report {
                Report::ScanFinished(scan) => progress.scan_finished(&scan),
                Report::FileStarted(path) => progress.file_started(&path),
                Report::FileFinished(report) => {
                    progress.file_finished(&report);
                    files.push(report);
                }
                Report::GoOn(answer) => answer.give(progress.should_continue()),
                Report::Ended(mut end) => {
                    if let Ok(summary) = &mut end.result {
                        summary.files = files;
                    }
                    return Some(end);
                }
            }
        }
        None
    }
}
