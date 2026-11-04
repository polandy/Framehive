// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import run: scans a source, makes the import plan, resolves conflicts and
//! copies each file with a verified copy (ADR 0020 §1, IMPORT-004). Planning and
//! execution are separate steps, so a caller can preview a plan before it runs.

use std::path::Path;

use domain::{Clock, LocalDateTime, Timestamp};
use files::{FileSystem, Hasher};
use metadata::{CaptureTimeError, NoCaptureTime, read_capture_time};
use naming::{CompatibilityMode, CounterDate, NameTemplate, SequenceCounters};

use crate::conflicts::{ConflictPolicy, TargetResolver};
use crate::job_code::{JobCode, require_job_code};
use crate::plan::{AssetFile, PlanNotice, PlannedFile};
use crate::report::{FileError, ImportSummary, ProgressSink, RunError};
use crate::scan::{FoundFile, LeftOut, SourceScan, scan_source};
use crate::sequence::{Reservation, SessionSequence, sequence_range};

mod copy;
mod rename;

use rename::{import_plan, rename};

/// What to import, from where to where, and how.
#[derive(Debug, Clone, Copy)]
pub struct ImportSettings<'a> {
    /// The folder to import from, absolute and without `..`.
    pub source: &'a Path,
    /// The import destination, absolute and without `..`.
    pub destination: &'a Path,
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
/// next one (IMPORT-004.11).
pub struct ImportRun<'a> {
    file_system: &'a dyn FileSystem,
    hasher: &'a dyn Hasher,
    clock: &'a dyn Clock,
    counters: &'a dyn SequenceCounters,
    session: &'a SessionSequence,
}

/// An import run that is planned and not yet executed: the selected files of a source
/// scan, each dated and named by one import plan.
///
/// [`ImportRun::plan`] and [`ImportRun::plan_first`] make it with the sequence
/// numbers it would get; [`ImportRun::start`] reserves them, and
/// [`ImportRun::execute`] executes it. Making it reads capture times, but resolves no
/// conflicts and writes nothing.
#[derive(Debug)]
pub struct PlannedRun {
    files: Vec<RunFile>,
    left_out: Vec<LeftOut>,
    notices: Vec<PlanNotice>,
    naming: PlanInputs,
}

/// What the import plan of a [`PlannedRun`] was made from, so that [`ImportRun::start`]
/// can name its files again with the reserved numbers.
#[derive(Debug)]
struct PlanInputs {
    /// The dated files, in scan order; each is its own asset until grouping exists.
    assets: Vec<AssetFile>,
    import_time: LocalDateTime,
    first_sequence_number: u32,
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

    /// What the person should know before the plan runs (IMPORT-002, "Files already
    /// imported").
    #[must_use]
    pub fn notices(&self) -> &[PlanNotice] {
        &self.notices
    }
}

/// One file of a [`PlannedRun`].
#[derive(Debug)]
pub struct RunFile {
    /// The file as the scan found it.
    pub found: FoundFile,
    /// Why the file has no capture time, when it is dated by its modification time.
    pub no_capture_time: Option<NoCaptureTime>,
    /// The file's entry in the import plan, or why the file could not be dated.
    pub planned: Result<PlannedFile, FileError>,
}

/// A found file with its times read, ready for the import plan.
struct DatedFile {
    asset_file: AssetFile,
    no_capture_time: Option<NoCaptureTime>,
}

impl<'a> ImportRun<'a> {
    /// Creates a run that works through these ports. `counters` holds the persisted
    /// sequence counters, and `session` the session counter of this run of the
    /// application (IMPORT-002).
    #[must_use]
    pub fn new(
        file_system: &'a dyn FileSystem,
        hasher: &'a dyn Hasher,
        clock: &'a dyn Clock,
        counters: &'a dyn SequenceCounters,
        session: &'a SessionSequence,
    ) -> Self {
        ImportRun {
            file_system,
            hasher,
            clock,
            counters,
            session,
        }
    }

    /// Imports every file of `settings.source` that the scan finds, in scan order: the
    /// composition of the scan, [`ImportRun::plan`], [`ImportRun::start`] and
    /// [`ImportRun::execute`].
    ///
    /// # Errors
    ///
    /// Returns a [`RunError`] if the template uses `{job}` without a job code, the
    /// source cannot be scanned, the current time cannot be shown in local time or
    /// the import cannot get its sequence numbers; then nothing is copied. A file that
    /// fails is reported in the [`ImportSummary`] instead.
    pub fn run(
        &self,
        settings: &ImportSettings<'_>,
        progress: &mut dyn ProgressSink,
    ) -> Result<ImportSummary, RunError> {
        // The job code and the import time are checked before the scan, so a run that
        // cannot start does not scan the source.
        require_job_code(settings.template, settings.job_code)?;
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
    /// only, and makes one import plan from `settings.source`, `template`, `job_code`
    /// and `mode`. Its sequence numbers are the ones the import would get if no other
    /// import started before it (IMPORT-002, "Reservation"). It reserves no numbers,
    /// resolves no conflicts and writes nothing, so it also serves a preview of the
    /// targets.
    ///
    /// # Errors
    ///
    /// Returns a [`RunError`] if the template uses `{job}` without a job code, the
    /// current time cannot be shown in local time, the persisted counters cannot be
    /// read, or the numbers do not fit the template. A file that cannot be dated is
    /// marked in the [`PlannedRun`] instead.
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
    /// the plan showed (IMPORT-002, "Reservation"). A dry run reserves nothing.
    ///
    /// # Errors
    ///
    /// Returns [`RunError::Sequence`] if the persisted counters cannot be read or
    /// stored, or the numbers do not fit the template; then nothing is reserved and
    /// the import must not run.
    pub fn start(
        &self,
        settings: &ImportSettings<'_>,
        mut plan: PlannedRun,
    ) -> Result<PlannedRun, RunError> {
        if settings.dry_run {
            return Ok(plan);
        }
        let first =
            self.first_sequence_number(settings.template, &plan.naming, Reservation::Reserve)?;
        if first != plan.naming.first_sequence_number {
            plan.naming.first_sequence_number = first;
            rename(&mut plan, settings);
        }
        Ok(plan)
    }

    /// Executes `plan`: resolves the conflicts of each planned file against
    /// `settings.destination` under `settings.policy`, and copies it, or with
    /// `settings.dry_run` only reports where it would go. Files run in plan order, and
    /// each is reported to `progress`. Before each file, `progress` is asked whether to
    /// go on; when it says stop, the remaining files are not started (IMPORT-004.13).
    ///
    /// `settings.source` must be the one the plan was made from.
    pub fn execute(
        &self,
        settings: &ImportSettings<'_>,
        plan: PlannedRun,
        progress: &mut dyn ProgressSink,
    ) -> ImportSummary {
        let mut resolver = TargetResolver::new(
            self.file_system,
            self.hasher,
            settings.source,
            settings.destination,
            settings.policy,
        );
        let mut files = Vec::with_capacity(plan.files.len());
        let mut stopped = false;
        for file in plan.files {
            if !progress.should_continue() {
                stopped = true;
                break;
            }
            progress.file_started(&file.found.path);
            let report = self.execute_file(settings, &mut resolver, file);
            progress.file_finished(&report);
            files.push(report);
        }
        ImportSummary {
            files,
            left_out: plan.left_out,
            stopped,
        }
    }

    /// The import time: now, in the machine's local time.
    fn import_time(&self) -> Result<LocalDateTime, RunError> {
        self.local_time(self.clock.now())
            .ok_or(RunError::ImportTimeOutOfRange)
    }

    /// The first sequence number of the import of `naming`, shown or reserved.
    fn first_sequence_number(
        &self,
        template: &NameTemplate,
        naming: &PlanInputs,
        reservation: Reservation,
    ) -> Result<u32, RunError> {
        let count = u32::try_from(naming.assets.len()).unwrap_or(u32::MAX);
        let today = CounterDate::of(naming.import_time);
        let range = sequence_range(
            self.counters,
            self.session,
            template,
            today,
            count,
            reservation,
        )?;
        Ok(range.map_or(1, |range| range.first))
    }

    /// Dates `files` and makes their import plan at `import_time`, with the sequence
    /// numbers the import would get.
    fn plan_at(
        &self,
        settings: &ImportSettings<'_>,
        files: Vec<FoundFile>,
        left_out: Vec<LeftOut>,
        import_time: LocalDateTime,
    ) -> Result<PlannedRun, RunError> {
        let dated: Vec<Result<DatedFile, FileError>> = files
            .iter()
            .map(|file| self.date(settings.source, file))
            .collect();
        let mut naming = PlanInputs {
            assets: dated
                .iter()
                .flatten()
                .map(|file| file.asset_file.clone())
                .collect(),
            import_time,
            first_sequence_number: 1,
        };
        naming.first_sequence_number =
            self.first_sequence_number(settings.template, &naming, Reservation::Preview)?;
        let plan = import_plan(&naming, settings);
        // The plan has one entry for each dated file, in the same order.
        let mut planned = plan.files().iter();
        let files = files
            .into_iter()
            .zip(dated)
            .map(|(found, dated)| match dated {
                Ok(dated) => RunFile {
                    found,
                    no_capture_time: dated.no_capture_time,
                    planned: planned.next().cloned().ok_or(FileError::NotPlanned),
                },
                Err(error) => RunFile {
                    found,
                    no_capture_time: None,
                    planned: Err(error),
                },
            })
            .collect();
        Ok(PlannedRun {
            files,
            left_out,
            notices: plan.notices().to_vec(),
            naming,
        })
    }

    /// Reads the capture time and the local modification time of `file`.
    fn date(&self, source: &Path, file: &FoundFile) -> Result<DatedFile, FileError> {
        let file_name = file
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(FileError::NameNotUtf8)?;
        let (capture_time, no_capture_time) =
            match read_capture_time(self.file_system, &source.join(&file.path)) {
                Ok(capture_time) => (Some(capture_time), None),
                Err(CaptureTimeError::NoCaptureTime(reason)) => (None, Some(reason)),
                Err(CaptureTimeError::Io { error, .. }) => {
                    return Err(FileError::CannotBeRead { error });
                }
            };
        let modification_time = self
            .local_time(file.modification_time)
            .ok_or(FileError::ModificationTimeOutOfRange)?;
        Ok(DatedFile {
            asset_file: AssetFile {
                file_name: file_name.to_owned(),
                role: file.role,
                capture_time,
                modification_time,
            },
            no_capture_time,
        })
    }

    fn local_time(&self, instant: Timestamp) -> Option<LocalDateTime> {
        LocalDateTime::from_timestamp(instant, self.clock.local_offset(instant))
    }
}
