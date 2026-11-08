// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import run: scans a source, makes the import plan, resolves conflicts and
//! copies each file with a verified copy (ADR 0020 §1, IMPORT-004). Planning and
//! execution are separate steps, so a caller can preview a plan before it runs.

use std::path::Path;

use domain::{Clock, LocalDateTime};
use files::{FileSystem, FreeSpace, Hasher, ReadOnlyFileSystem};
use history::ImportHistory;
use metadata::NoCaptureTime;
use metadata::xmp::XmlSyntax;
use naming::SequenceCounters;

use crate::backup::BackupCounts;
use crate::destinations::{check_destinations, create_destinations};
use crate::free_space::{FreeSpaceUnknown, NotEnoughSpace};
use crate::report::{FileError, ImportSummary, ProgressSink, RunError};
use crate::scan::{FoundFile, LeftOut, SourceScan, scan_source};
use crate::sequence::SessionSequence;

mod copy;
mod dating;
mod free_space;
mod planning;
mod recognition;
mod rename;
mod resolvers;
mod settings;
mod source;
mod steps;

use copy::Sidecars;
pub use planning::FilePlan;
use planning::PlanInputs;
use recognition::{Recognition, RecordedRun};
use resolvers::{ReservedBy, Resolvers};
pub use settings::ImportSettings;
use steps::Keeping;
pub(crate) use steps::RunSteps;

/// The execution of an import plan (glossary: Import run).
///
/// It runs on the caller's thread, through the ports only, and reports each step to a
/// [`ProgressSink`]. Each file the scan finds is its own asset until grouping
/// (LIBRARY-001) exists. A file that fails is reported and the run goes on with the
/// next one (IMPORT-004.11). Each verified copy is recorded in the import history,
/// which recognizes the files imported before when the plan is made (IMPORT-009).
pub struct ImportRun<'a> {
    /// Reads the source, which the run never writes (ADR 0034): the file system of
    /// the destinations, unless [`ImportRun::with_source`] gives one of its own.
    source_file_system: &'a dyn ReadOnlyFileSystem,
    /// Whether the source lies on a file system of its own, which no destination can
    /// lie inside.
    separate_source: bool,
    /// How the import history names the source, if not by `settings.source`.
    source_name: Option<&'a Path>,
    /// Reads and writes the destinations.
    file_system: &'a dyn FileSystem,
    hasher: &'a dyn Hasher,
    clock: &'a dyn Clock,
    counters: &'a dyn SequenceCounters,
    session: &'a SessionSequence,
    history: &'a dyn ImportHistory,
    free_space: &'a dyn FreeSpace,
    /// The target names other import jobs reserved (IMPORT-015).
    reserved_names: Option<ReservedBy<'a>>,
    /// Reads and writes the XML of sidecars (IMPORT-019); `None` until
    /// [`ImportRun::with_xml_syntax`] gives one.
    xml_syntax: Option<&'a dyn XmlSyntax>,
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
            source_file_system: file_system,
            separate_source: false,
            source_name: None,
            file_system,
            hasher,
            clock,
            counters,
            session,
            history,
            free_space,
            reserved_names: None,
            xml_syntax: None,
        }
    }

    /// Writes the XML of sidecars through `xml_syntax` (IMPORT-019).
    #[must_use]
    pub fn with_xml_syntax(mut self, xml_syntax: &'a dyn XmlSyntax) -> Self {
        self.xml_syntax = Some(xml_syntax);
        self
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
        let mut keeping = Keeping::new(progress);
        let summary = self.run_steps(settings, &mut keeping)?;
        Ok(keeping.into_summary(summary))
    }

    /// [`ImportRun::run`], handing each file report over to `steps` by value; the
    /// summary it returns holds no file reports.
    pub(crate) fn run_steps(
        &self,
        settings: &ImportSettings<'_>,
        steps: &mut dyn RunSteps,
    ) -> Result<ImportSummary, RunError> {
        // The job code and the import time are checked before the scan, so a run that
        // cannot start does not scan the source.
        settings.check_templates()?;
        check_destinations(settings, self.checked_source(settings))?;
        let import_time = self.import_time()?;
        let scan = scan_source(
            self.source_file_system,
            settings.source,
            settings.destination,
        )?;
        steps.scan_finished(&scan);
        let plan = self.plan_at(settings, scan.files, scan.left_out, import_time)?;
        let plan = self.start(settings, plan)?;
        Ok(self.execute_steps(settings, plan, steps))
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
        settings.check_templates()?;
        let import_time = self.import_time()?;
        let selected = scan.files[..count.min(scan.files.len())].to_vec();
        self.plan_at(settings, selected, scan.left_out.clone(), import_time)
    }

    /// Starts the import of `plan`: reserves its sequence numbers before the first
    /// file is copied, unless they are reserved already, and names its files again if
    /// another import took the numbers the plan showed (IMPORT-002, "Reservation"). Then it records the start of the
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
    /// [`RunError::NotEnoughSpace`] if a file system is short,
    /// [`RunError::DestinationNotCreated`] if a missing destination cannot be created,
    /// and
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
        check_destinations(settings, self.checked_source(settings))?;
        let check = self.check_free_space(settings, &plan);
        if !check.shortages.is_empty() {
            return Err(RunError::NotEnoughSpace(NotEnoughSpace {
                shortages: check.shortages,
            }));
        }
        plan.free_space_unknown = check.unknown;
        let carries_sidecars = plan
            .files
            .iter()
            .any(|file| !file.found.sidecars.is_empty());
        if (settings.writes_sidecars() || carries_sidecars) && self.xml_syntax.is_none() {
            return Err(RunError::SidecarsWithoutXmlSyntax);
        }
        if settings.dry_run {
            return Ok(plan);
        }
        create_destinations(self.file_system, settings)?;
        let mut plan = self.reserve(settings, plan)?;
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
        let mut keeping = Keeping::new(progress);
        let summary = self.execute_steps(settings, plan, &mut keeping);
        keeping.into_summary(summary)
    }

    /// [`ImportRun::execute`], handing each file report over to `steps` by value once
    /// the backup counts hold it; the summary it returns holds no file reports.
    pub(crate) fn execute_steps(
        &self,
        settings: &ImportSettings<'_>,
        plan: PlannedRun,
        steps: &mut dyn RunSteps,
    ) -> ImportSummary {
        let mut resolvers = Resolvers::new(self, settings);
        let sidecars = Sidecars::of_run(self, settings, plan.naming.import_time.year());
        let mut backups: Vec<BackupCounts> = settings
            .all_backup_destinations()
            .into_iter()
            .map(|destination| {
                let receives_photos = settings
                    .backup_destinations
                    .iter()
                    .any(|backup| backup == destination);
                BackupCounts::new(destination, receives_photos)
            })
            .collect();
        let mut stopped = false;
        let mut history_error = None;
        for file in plan.files {
            if !steps.should_continue() {
                stopped = true;
                break;
            }
            steps.file_started(&file.found.path);
            let (report, recorded) = self.execute_file(
                settings,
                &mut resolvers,
                sidecars.as_ref(),
                file,
                plan.recorded,
            );
            for counts in &mut backups {
                counts.count(report.backups.iter());
            }
            steps.file_finished(report);
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
        ImportSummary {
            files: Vec::new(),
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
