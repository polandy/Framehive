// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Executing one planned file of an import run: conflict resolution and the verified
//! copy (IMPORT-003, IMPORT-004).

use files::copy_verified;

use super::{ImportRun, ImportSettings, RunFile};
use crate::conflicts::{ConflictOutcome, TargetRequest, TargetResolver};
use crate::plan::PlannedFile;
use crate::report::{FileError, FileOutcome, FileReport};
use crate::scan::FoundFile;

impl ImportRun<'_> {
    /// Imports one file of a planned run and reports what happened to it.
    pub(super) fn execute_file(
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
