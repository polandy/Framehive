// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import run: scans a source, makes the import plan, resolves conflicts and
//! copies each file with a verified copy (ADR 0020 §1, IMPORT-004). Planning and
//! execution are separate steps, so a caller can preview a plan before it runs.

use std::path::Path;

use domain::{Clock, LocalDateTime, Timestamp};
use files::{FileSystem, Hasher, copy_verified};
use metadata::{CaptureTimeError, NoCaptureTime, read_capture_time};
use naming::{CompatibilityMode, NameTemplate};

use crate::conflicts::{ConflictOutcome, ConflictPolicy, TargetRequest, TargetResolver};
use crate::job_code::{JobCode, require_job_code};
use crate::plan::{AssetFile, ImportPlan, PlannedFile};
use crate::report::{FileError, FileOutcome, FileReport, ImportSummary, ProgressSink, RunError};
use crate::scan::{FoundFile, LeftOut, SourceScan, scan_source};

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
}

/// An import run that is planned and not yet executed: the selected files of a source
/// scan, each dated and named by one import plan.
///
/// [`ImportRun::plan`] and [`ImportRun::plan_first`] make it; [`ImportRun::execute`]
/// executes it. Making it reads capture times, but resolves no conflicts and writes
/// nothing.
#[derive(Debug)]
pub struct PlannedRun {
    files: Vec<RunFile>,
    left_out: Vec<LeftOut>,
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
    /// The file's entry in the import plan, or why the file could not be dated.
    pub planned: Result<PlannedFile, FileError>,
}

/// A found file with its times read, ready for the import plan.
struct DatedFile {
    asset_file: AssetFile,
    no_capture_time: Option<NoCaptureTime>,
}

impl<'a> ImportRun<'a> {
    /// Creates a run that works through these ports.
    #[must_use]
    pub fn new(
        file_system: &'a dyn FileSystem,
        hasher: &'a dyn Hasher,
        clock: &'a dyn Clock,
    ) -> Self {
        ImportRun {
            file_system,
            hasher,
            clock,
        }
    }

    /// Imports every file of `settings.source` that the scan finds, in scan order: the
    /// composition of the scan, [`ImportRun::plan`] and [`ImportRun::execute`].
    ///
    /// # Errors
    ///
    /// Returns a [`RunError`] if the template uses `{job}` without a job code, the
    /// source cannot be scanned or the current time cannot be shown in local time;
    /// then nothing is copied. A file that fails is
    /// reported in the [`ImportSummary`] instead.
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
        let plan = self.plan_at(settings, scan.files, scan.left_out, import_time);
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
    /// and `mode`. It resolves no conflicts and writes nothing, so it also serves a
    /// preview of the targets.
    ///
    /// # Errors
    ///
    /// Returns a [`RunError`] if the template uses `{job}` without a job code or the
    /// current time cannot be shown in local time. A file that cannot be dated is
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
        Ok(self.plan_at(settings, selected, scan.left_out.clone(), import_time))
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

    /// Dates `files` and makes their import plan at `import_time`.
    fn plan_at(
        &self,
        settings: &ImportSettings<'_>,
        files: Vec<FoundFile>,
        left_out: Vec<LeftOut>,
        import_time: LocalDateTime,
    ) -> PlannedRun {
        let dated: Vec<Result<DatedFile, FileError>> = files
            .iter()
            .map(|file| self.date(settings.source, file))
            .collect();
        let assets: Vec<Vec<AssetFile>> = dated
            .iter()
            .flatten()
            .map(|file| vec![file.asset_file.clone()])
            .collect();
        let plan = ImportPlan::new(
            &assets,
            settings.template,
            settings.mode,
            import_time,
            settings.job_code,
        );
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
        PlannedRun { files, left_out }
    }

    /// Imports one file of a planned run and reports what happened to it.
    fn execute_file(
        &self,
        settings: &ImportSettings<'_>,
        resolver: &mut TargetResolver<'_>,
        file: RunFile,
    ) -> FileReport {
        let outcome = match file.planned {
            Ok(planned) => self.import_planned(settings, resolver, &file.found, &planned),
            Err(error) => FileOutcome::Failed(error),
        };
        FileReport {
            path: file.found.path,
            dated_by_modification_time: file.no_capture_time,
            outcome,
        }
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

    /// Resolves the conflicts of one planned file and copies it.
    fn import_planned(
        &self,
        settings: &ImportSettings<'_>,
        resolver: &mut TargetResolver<'_>,
        file: &FoundFile,
        planned: &PlannedFile,
    ) -> FileOutcome {
        match self.try_import(settings, resolver, file, planned) {
            Ok(outcome) => outcome,
            Err(error) => FileOutcome::Failed(error),
        }
    }

    fn try_import(
        &self,
        settings: &ImportSettings<'_>,
        resolver: &mut TargetResolver<'_>,
        file: &FoundFile,
        planned: &PlannedFile,
    ) -> Result<FileOutcome, FileError> {
        let target = match &planned.target {
            Ok(target) => target,
            Err(error) => return Err(FileError::CannotBeNamed(error.clone())),
        };
        let request = TargetRequest {
            source_path: &file.path,
            size: file.size,
            target,
        };
        Ok(match resolver.resolve(&request)? {
            ConflictOutcome::CopyTo { target } if settings.dry_run => {
                FileOutcome::WouldCopy { target }
            }
            ConflictOutcome::CopyTo { target } => {
                let checksum = copy_verified(
                    self.file_system,
                    self.hasher,
                    &settings.source.join(&file.path),
                    &settings.destination.join(&target),
                )?;
                FileOutcome::Copied { target, checksum }
            }
            ConflictOutcome::AlreadyImported { target } => FileOutcome::AlreadyImported { target },
            ConflictOutcome::DuplicateOf { earlier } => FileOutcome::DuplicateOf { earlier },
            ConflictOutcome::SkippedNameConflict { target } => {
                FileOutcome::SkippedNameConflict { target }
            }
            ConflictOutcome::FailedNameConflict { target } => {
                return Err(FileError::NameConflict { target });
            }
        })
    }
}
