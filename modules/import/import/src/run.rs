// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import run: scans a source, makes the import plan, resolves conflicts and
//! copies each file with a verified copy (ADR 0020 §1, IMPORT-004).

use std::path::Path;

use domain::{Clock, LocalDateTime, Timestamp};
use files::{FileSystem, Hasher, copy_verified};
use metadata::{CaptureTimeError, NoCaptureTime, read_capture_time};
use naming::{CompatibilityMode, NameTemplate};

use crate::conflicts::{ConflictOutcome, ConflictPolicy, TargetRequest, TargetResolver};
use crate::job_code::{JobCode, require_job_code};
use crate::plan::{AssetFile, ImportPlan, PlannedFile};
use crate::report::{FileError, FileOutcome, FileReport, ImportSummary, ProgressSink, RunError};
use crate::scan::{FoundFile, scan_source};

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

    /// Imports every file of `settings.source` that the scan finds, in scan order.
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
        require_job_code(settings.template, settings.job_code)?;
        let import_time = self
            .local_time(self.clock.now())
            .ok_or(RunError::ImportTimeOutOfRange)?;
        let scan = scan_source(self.file_system, settings.source, settings.destination)?;
        progress.scan_finished(&scan);

        let dated: Vec<Result<DatedFile, FileError>> = scan
            .files
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
        let mut planned = plan.files().iter();

        let mut resolver = TargetResolver::new(
            self.file_system,
            self.hasher,
            settings.source,
            settings.destination,
            settings.policy,
        );
        let mut files = Vec::with_capacity(scan.files.len());
        for (file, dated) in scan.files.iter().zip(dated) {
            progress.file_started(&file.path);
            let (dated_by_modification_time, outcome) = match dated {
                Ok(dated) => (
                    dated.no_capture_time,
                    self.import_planned(settings, &mut resolver, file, planned.next()),
                ),
                Err(error) => (None, FileOutcome::Failed(error)),
            };
            let report = FileReport {
                path: file.path.clone(),
                dated_by_modification_time,
                outcome,
            };
            progress.file_finished(&report);
            files.push(report);
        }
        Ok(ImportSummary {
            files,
            left_out: scan.left_out,
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

    /// Resolves the conflicts of one planned file and copies it.
    fn import_planned(
        &self,
        settings: &ImportSettings<'_>,
        resolver: &mut TargetResolver<'_>,
        file: &FoundFile,
        planned: Option<&PlannedFile>,
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
        planned: Option<&PlannedFile>,
    ) -> Result<FileOutcome, FileError> {
        // The plan has one entry for each dated file, in the same order.
        let target = match planned.map(|planned| &planned.target) {
            Some(Ok(target)) => target,
            Some(Err(error)) => return Err(FileError::CannotBeNamed(error.clone())),
            None => return Err(FileError::NotPlanned),
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
