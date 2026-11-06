// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import run: scans a source, makes the import plan, resolves conflicts and
//! copies each file with a verified copy (ADR 0020 §1, IMPORT-004). Planning and
//! execution are separate steps, so a caller can preview a plan before it runs.

use std::path::{Path, PathBuf};

use domain::{Clock, LocalDateTime};
use files::{FileSystem, FreeSpace, Hasher};
use history::ImportHistory;
use metadata::NoCaptureTime;
use naming::{CompatibilityMode, NameTemplate, SequenceCounters};

use crate::backup::BackupCounts;
use crate::conflicts::ConflictPolicy;
use crate::destinations::check_destinations;
use crate::free_space::{FreeSpaceUnknown, NotEnoughSpace};
use crate::job_code::{JobCode, require_job_code};
use crate::plan::PlannedFile;
use crate::report::{FileError, ImportSummary, ProgressSink, RunError};
use crate::scan::{FoundFile, LeftOut, SourceScan, scan_source};
use crate::sequence::{Reservation, SessionSequence};

mod copy;
mod dating;
mod free_space;
mod planning;
mod recognition;
mod rename;
mod resolvers;

use planning::PlanInputs;
use recognition::{Recognition, RecordedRun};
use rename::rename;
use resolvers::Resolvers;

/// What to import, from where to where, and how.
#[derive(Debug, Clone, Copy)]
pub struct ImportSettings<'a> {
    /// The folder to import from, absolute and without `..`.
    pub source: &'a Path,
    /// The import destination, absolute and without `..`.
    pub destination: &'a Path,
    /// The backup destinations, absolute and without `..`, in the order they are
    /// used (IMPORT-007). Each file goes to the same target path below each of them.
    pub backup_destinations: &'a [PathBuf],
    /// The video destination, absolute and without `..`: where videos are copied to
    /// (IMPORT-013). `None` copies them to the import destination.
    pub video_destination: Option<&'a Path>,
    /// The video backup destinations, absolute and without `..`, in the order they
    /// are used (IMPORT-013). Empty backs videos up to the backup destinations, so
    /// videos are never left without a backup while photos have one.
    pub video_backup_destinations: &'a [PathBuf],
    /// The name template for the target paths (IMPORT-001).
    pub template: &'a NameTemplate,
    /// The job code for `{job}` (IMPORT-006); required when the template uses it.
    pub job_code: Option<&'a JobCode>,
    /// Whether target names are kept compatible with every file system (IMPORT-001).
    pub mode: CompatibilityMode,
    /// What happens when a target name holds a different file (IMPORT-003).
    pub policy: ConflictPolicy,
    /// Plan and resolve conflicts, but write nothing (CLI-001, `--dry-run`).
    pub dry_run: bool,
}

/// The execution of an import plan (glossary: Import run).
///
/// It runs on the caller's thread, through the ports only, and reports each step to a
/// [`ProgressSink`]. Each file the scan finds is its own asset until grouping
/// (LIBRARY-001) exists. A file that fails is reported and the run goes on with the
/// next one (IMPORT-004.11). Each verified copy is recorded in the import history,
/// which recognizes the files imported before when the plan is made (IMPORT-009).
pub struct ImportRun<'a> {
    file_system: &'a dyn FileSystem,
    hasher: &'a dyn Hasher,
    clock: &'a dyn Clock,
    counters: &'a dyn SequenceCounters,
    session: &'a SessionSequence,
    history: &'a dyn ImportHistory,
    free_space: &'a dyn FreeSpace,
}

/// An import run that is planned and not yet executed: the selected files of a source
/// scan, each dated and named by one import plan.
///
/// [`ImportRun::plan`] and [`ImportRun::plan_first`] make it with the sequence
/// numbers it would get; [`ImportRun::start`] reserves them and records the start of
/// the run in the import history, and [`ImportRun::execute`] executes it. Making it
/// reads capture times and the import history, and the content of the files the
/// history may hold, but resolves no conflicts and writes nothing.
#[derive(Debug)]
pub struct PlannedRun {
    files: Vec<RunFile>,
    left_out: Vec<LeftOut>,
    naming: PlanInputs,
    /// The run in the import history, once it is started and not a dry run.
    recorded: Option<RecordedRun>,
    /// The destinations whose free space the start could not read (IMPORT-012).
    free_space_unknown: Vec<FreeSpaceUnknown>,
}

impl PlannedRun {
    /// The selected files, in scan order.
    #[must_use]
    pub fn files(&self) -> &[RunFile] {
        &self.files
    }

    /// The files and folders the scan left out.
    #[must_use]
    pub fn left_out(&self) -> &[LeftOut] {
        &self.left_out
    }
}

/// One file of a [`PlannedRun`].
#[derive(Debug)]
pub struct RunFile {
    /// The file as the scan found it.
    pub found: FoundFile,
    /// Why the file has no capture time, when it is dated by its modification time.
    pub no_capture_time: Option<NoCaptureTime>,
    /// The file's entry in the import plan, or that it is imported before, or why the
    /// file could not be dated or looked up.
    pub planned: Result<FilePlan, FileError>,
    /// What the import history records the file by; `None` when it could not be
    /// dated.
    recognition: Option<Recognition>,
}

/// What the plan decided for one file of a [`PlannedRun`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilePlan {
    /// Named by the import plan; copied, or found at its target, when the run executes.
    Named(PlannedFile),
    /// The import history holds a verified copy at the file's import destination, the
    /// video destination for a video (IMPORT-013): the file is not named, gets no
    /// sequence number and is not copied (IMPORT-009).
    ImportedBefore {
        /// The target of the most recent recorded copy, relative to the import
        /// destination.
        target: String,
    },
}

impl FilePlan {
    /// The file's entry in the import plan, unless it is imported before.
    #[must_use]
    pub fn named(&self) -> Option<&PlannedFile> {
        match self {
            FilePlan::Named(planned) => Some(planned),
            FilePlan::ImportedBefore { .. } => None,
        }
    }
}

impl<'a> ImportRun<'a> {
    /// Creates a run that works through these ports. `counters` holds the persisted
    /// sequence counters, and `session` the session counter of this run of the
    /// application (IMPORT-002); `history` is the import history of this machine
    /// (IMPORT-009), and `free_space` tells how much the destinations can hold
    /// (IMPORT-012).
    #[must_use]
    pub fn new(
        file_system: &'a dyn FileSystem,
        hasher: &'a dyn Hasher,
        clock: &'a dyn Clock,
        counters: &'a dyn SequenceCounters,
        session: &'a SessionSequence,
        history: &'a dyn ImportHistory,
        free_space: &'a dyn FreeSpace,
    ) -> Self {
        ImportRun {
            file_system,
            hasher,
            clock,
            counters,
            session,
            history,
            free_space,
        }
    }

    /// Imports every file of `settings.source` that the scan finds, in scan order: the
    /// composition of the scan, [`ImportRun::plan`], [`ImportRun::start`] and
    /// [`ImportRun::execute`].
    ///
    /// # Errors
    ///
    /// Returns a [`RunError`] if the template uses `{job}` without a job code, a
    /// backup destination, the video destination or a video backup destination is
    /// invalid, a destination does not have enough space, the
    /// source cannot be scanned, the current time
    /// cannot be shown in local time, the import cannot get its sequence numbers or
    /// the import history cannot be used; then nothing is copied. A file that
    /// fails is reported in the [`ImportSummary`] instead.
    pub fn run(
        &self,
        settings: &ImportSettings<'_>,
        progress: &mut dyn ProgressSink,
    ) -> Result<ImportSummary, RunError> {
        // The job code and the import time are checked before the scan, so a run that
        // cannot start does not scan the source.
        require_job_code(settings.template, settings.job_code)?;
        check_destinations(settings)?;
        let import_time = self.import_time()?;
        let scan = scan_source(self.file_system, settings.source, settings.destination)?;
        progress.scan_finished(&scan);
        let plan = self.plan_at(settings, scan.files, scan.left_out, import_time)?;
        let plan = self.start(settings, plan)?;
        Ok(self.execute(settings, plan, progress))
    }

    /// Plans every file of `scan`: see [`ImportRun::plan_first`].
    ///
    /// # Errors
    ///
    /// As [`ImportRun::plan_first`].
    pub fn plan(
        &self,
        settings: &ImportSettings<'_>,
        scan: &SourceScan,
    ) -> Result<PlannedRun, RunError> {
        self.plan_first(settings, scan, scan.files.len())
    }

    /// Plans the first `count` files of `scan`, or all of them if it has fewer.
    ///
    /// It fixes the import time once, reads the capture time of each selected file
    /// only, looks each one up in the import history, and makes one import plan from
    /// `settings.source`, `template`, `job_code` and `mode` for the files that are not
    /// imported before (IMPORT-009). Its sequence numbers are the ones the import would get if no other
    /// import started before it (IMPORT-002, "Reservation"). It reserves no numbers,
    /// resolves no conflicts and writes nothing, so it also serves a preview of the
    /// targets.
    ///
    /// # Errors
    ///
    /// Returns a [`RunError`] if the template uses `{job}` without a job code, the
    /// current time cannot be shown in local time, the persisted counters cannot be
    /// read, the numbers do not fit the template, or the import history cannot be
    /// read. A file that cannot be dated or read is marked in the [`PlannedRun`]
    /// instead.
    pub fn plan_first(
        &self,
        settings: &ImportSettings<'_>,
        scan: &SourceScan,
        count: usize,
    ) -> Result<PlannedRun, RunError> {
        require_job_code(settings.template, settings.job_code)?;
        let import_time = self.import_time()?;
        let selected = scan.files[..count.min(scan.files.len())].to_vec();
        self.plan_at(settings, selected, scan.left_out.clone(), import_time)
    }

    /// Starts the import of `plan`: reserves its sequence numbers before the first
    /// file is copied, and names its files again if another import took the numbers
    /// the plan showed (IMPORT-002, "Reservation"). Then it records the start of the
    /// run in the import history (IMPORT-009). A dry run reserves and records nothing.
    /// It checks the backup destinations, the video destination and the video backup
    /// destinations first (IMPORT-007.11, IMPORT-013.8), and then that each file
    /// system can hold what the run writes to it, also in a dry run (IMPORT-012).
    /// Destinations whose free space cannot be read are kept for the
    /// [`ImportSummary`], and the run goes on.
    ///
    /// # Errors
    ///
    /// Returns [`RunError::BackupDestination`], [`RunError::VideoDestination`] or
    /// [`RunError::VideoBackupDestination`] if a destination is invalid,
    /// [`RunError::NotEnoughSpace`] if a file system is short, and
    /// [`RunError::Sequence`] if the persisted counters cannot be read or
    /// stored, or the numbers do not fit the template; then nothing is reserved and
    /// the import must not run. Returns [`RunError::History`] if the import history
    /// cannot record the start, and [`RunError::ImportTimeOutOfRange`] if the current
    /// time cannot be recorded; then the import must not run either.
    pub fn start(
        &self,
        settings: &ImportSettings<'_>,
        mut plan: PlannedRun,
    ) -> Result<PlannedRun, RunError> {
        check_destinations(settings)?;
        let check = self.check_free_space(settings, &plan);
        if !check.shortages.is_empty() {
            return Err(RunError::NotEnoughSpace(NotEnoughSpace {
                shortages: check.shortages,
            }));
        }
        plan.free_space_unknown = check.unknown;
        if settings.dry_run {
            return Ok(plan);
        }
        let first =
            self.first_sequence_number(settings.template, &plan.naming, Reservation::Reserve)?;
        if first != plan.naming.first_sequence_number {
            plan.naming.first_sequence_number = first;
            rename(&mut plan, settings);
        }
        plan.recorded = Some(self.start_recorded_run(settings)?);
        Ok(plan)
    }

    /// Executes `plan`: resolves the conflicts of each planned file against its own
    /// destination (IMPORT-013) under `settings.policy`, and copies it, or with
    /// `settings.dry_run` only reports where it would go. Files run in plan order, and
    /// each is reported to `progress`. Before each file, `progress` is asked whether to
    /// go on; when it says stop, the remaining files are not started (IMPORT-004.13).
    /// Each verified copy is recorded in the import history; when it cannot be
    /// recorded, the run stops after that file (IMPORT-009.16). The end of a started
    /// run is recorded, also when it stopped.
    ///
    /// `settings.source` must be the one the plan was made from.
    pub fn execute(
        &self,
        settings: &ImportSettings<'_>,
        plan: PlannedRun,
        progress: &mut dyn ProgressSink,
    ) -> ImportSummary {
        let mut resolvers = Resolvers::new(self, settings);
        let mut files = Vec::with_capacity(plan.files.len());
        let mut stopped = false;
        let mut history_error = None;
        for file in plan.files {
            if !progress.should_continue() {
                stopped = true;
                break;
            }
            progress.file_started(&file.found.path);
            let (report, recorded) =
                self.execute_file(settings, &mut resolvers, file, plan.recorded);
            progress.file_finished(&report);
            files.push(report);
            if let Err(error) = recorded {
                history_error = Some(error);
                break;
            }
        }
        if let Some(run) = plan.recorded
            && let Err(error) = self.finish_recorded_run(run)
        {
            history_error.get_or_insert(error);
        }
        let backups = settings
            .all_backup_destinations()
            .into_iter()
            .map(|destination| {
                BackupCounts::of(destination, files.iter().flat_map(|file| &file.backups))
            })
            .collect();
        ImportSummary {
            files,
            left_out: plan.left_out,
            stopped,
            dry_run: settings.dry_run,
            history_error,
            backups,
            free_space_unknown: plan.free_space_unknown,
        }
    }

    /// The import time: now, in the machine's local time.
    fn import_time(&self) -> Result<LocalDateTime, RunError> {
        self.local_time(self.clock.now())
            .ok_or(RunError::ImportTimeOutOfRange)
    }
}
