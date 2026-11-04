// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Executing one planned file of an import run: conflict resolution, the verified
//! copy and the backups (IMPORT-003, IMPORT-004, IMPORT-007).

use std::io;
use std::path::{Path, PathBuf};

use domain::Checksum;
use files::{CopyError, CopyErrorKind, CopyStep, checksum_file, copy_verified_to_all};

use super::{ImportRun, ImportSettings, RunFile};
use crate::backup::{
    BackupError, BackupOutcome, BackupReport, BackupTarget, inspect_backup_target,
};
use crate::conflicts::{ConflictError, ConflictOutcome, TargetRequest, TargetResolver};
use crate::plan::PlannedFile;
use crate::report::{FileError, FileOutcome, FileReport};
use crate::scan::FoundFile;

/// What one file gets: its outcome at the import destination and at each backup
/// destination.
type Imported = (FileOutcome, Vec<BackupReport>);

impl ImportRun<'_> {
    /// Imports one file of a planned run and reports what happened to it.
    pub(super) fn execute_file(
        &self,
        settings: &ImportSettings<'_>,
        resolver: &mut TargetResolver<'_>,
        file: RunFile,
    ) -> FileReport {
        let (outcome, backups) = match file.planned {
            Ok(planned) => self.import_planned(settings, resolver, &file.found, &planned),
            Err(error) => (FileOutcome::Failed(error), Vec::new()),
        };
        FileReport {
            path: file.found.path,
            dated_by_modification_time: file.no_capture_time,
            outcome,
            backups,
        }
    }

    /// Resolves the conflicts of one planned file, copies it and backs it up.
    fn import_planned(
        &self,
        settings: &ImportSettings<'_>,
        resolver: &mut TargetResolver<'_>,
        file: &FoundFile,
        planned: &PlannedFile,
    ) -> Imported {
        let target = match &planned.target {
            Ok(target) => target,
            Err(error) => return failed(FileError::CannotBeNamed(error.clone())),
        };
        let request = TargetRequest {
            source_path: &file.path,
            size: file.size,
            target,
        };
        let outcome = match resolver.resolve(&request) {
            Ok(outcome) => outcome,
            Err(error) => return failed(FileError::CannotCheckTarget(error)),
        };
        match outcome {
            ConflictOutcome::CopyTo { target } if settings.dry_run => {
                (FileOutcome::WouldCopy { target }, Vec::new())
            }
            ConflictOutcome::CopyTo { target } => {
                let copy = FileCopy::new(settings, file, &target);
                let (copied, backups) = self.copy_file(&copy, resolver, true);
                let outcome = match copied {
                    Some(Ok(checksum)) => FileOutcome::Copied { target, checksum },
                    Some(Err(error)) => FileOutcome::Failed(FileError::Copy(error)),
                    None => FileOutcome::Failed(FileError::Copy(unfinished(&copy.import_target))),
                };
                (outcome, backups)
            }
            // A file already imported is backed up, so that a re-run fills a new or a
            // failed backup destination (IMPORT-007).
            ConflictOutcome::AlreadyImported { target } if !settings.dry_run => {
                let copy = FileCopy::new(settings, file, &target);
                let (_, backups) = self.copy_file(&copy, resolver, false);
                (FileOutcome::AlreadyImported { target }, backups)
            }
            ConflictOutcome::AlreadyImported { target } => {
                (FileOutcome::AlreadyImported { target }, Vec::new())
            }
            ConflictOutcome::DuplicateOf { earlier } => {
                (FileOutcome::DuplicateOf { earlier }, Vec::new())
            }
            ConflictOutcome::SkippedNameConflict { target } => {
                (FileOutcome::SkippedNameConflict { target }, Vec::new())
            }
            ConflictOutcome::FailedNameConflict { target } => {
                failed(FileError::NameConflict { target })
            }
        }
    }

    /// Copies the file with one read of the source: to the import destination when
    /// `to_import_destination`, and to each backup target that is free. Returns the
    /// result at the import destination, if it was a target, and one report per
    /// backup destination.
    fn copy_file(
        &self,
        copy: &FileCopy<'_>,
        resolver: &mut TargetResolver<'_>,
        to_import_destination: bool,
    ) -> (Option<Result<Checksum, CopyError>>, Vec<BackupReport>) {
        let found: Vec<BackupTarget> = copy
            .backup_targets
            .iter()
            .map(|path| inspect_backup_target(self.file_system, path, copy.size))
            .collect();
        let mut targets: Vec<&Path> = Vec::new();
        if to_import_destination {
            targets.push(&copy.import_target);
        }
        for (path, found) in copy.backup_targets.iter().zip(&found) {
            if matches!(found, BackupTarget::Free) {
                targets.push(path);
            }
        }
        let copies = copy_verified_to_all(self.file_system, self.hasher, &copy.source, &targets);
        let mut results = copies.targets.into_iter();
        let imported = if to_import_destination {
            results.next()
        } else {
            None
        };
        let mut source_checksum = copies.source_checksum;
        let backups = copy
            .destinations
            .iter()
            .zip(&copy.backup_targets)
            .zip(found)
            .map(|((destination, path), found)| {
                let outcome = match found {
                    BackupTarget::Free => match results.next() {
                        Some(Ok(_)) => BackupOutcome::BackedUp,
                        Some(Err(error)) => BackupOutcome::Failed(BackupError::Copy(error)),
                        None => BackupOutcome::Failed(BackupError::Copy(unfinished(path))),
                    },
                    BackupTarget::SameSize => {
                        self.compare_backup(copy, resolver, &mut source_checksum, path)
                    }
                    BackupTarget::Failed(error) => BackupOutcome::Failed(error),
                };
                BackupReport {
                    destination: destination.clone(),
                    outcome,
                }
            })
            .collect();
        (imported, backups)
    }

    /// Compares a file of the source's size at the backup target `path` with the
    /// source: "already backed up" or a name conflict. The source's checksum is the
    /// copy's, or read once through the resolver when no copy read the source.
    fn compare_backup(
        &self,
        copy: &FileCopy<'_>,
        resolver: &mut TargetResolver<'_>,
        source_checksum: &mut Option<Checksum>,
        path: &Path,
    ) -> BackupOutcome {
        let ours = match source_checksum {
            Some(checksum) => checksum.clone(),
            None => match resolver.source_checksum(copy.source_path) {
                Ok(checksum) => source_checksum.insert(checksum).clone(),
                Err(error) => return BackupOutcome::Failed(BackupError::CannotCheckTarget(error)),
            },
        };
        match checksum_file(self.file_system, self.hasher, path) {
            Ok(theirs) if theirs == ours => BackupOutcome::AlreadyBackedUp,
            Ok(_) => BackupOutcome::Failed(BackupError::NameConflict {
                path: path.to_path_buf(),
            }),
            Err(source) => BackupOutcome::Failed(BackupError::CannotCheckTarget(ConflictError {
                path: path.to_path_buf(),
                source,
            })),
        }
    }
}

/// The paths of one file's copy: its source, its import target and its backup
/// targets, all for the same target path.
struct FileCopy<'a> {
    source_path: &'a Path,
    source: PathBuf,
    size: u64,
    import_target: PathBuf,
    destinations: &'a [PathBuf],
    backup_targets: Vec<PathBuf>,
}

impl<'a> FileCopy<'a> {
    fn new(settings: &ImportSettings<'a>, file: &'a FoundFile, target: &str) -> Self {
        FileCopy {
            source_path: &file.path,
            source: settings.source.join(&file.path),
            size: file.size,
            import_target: settings.destination.join(target),
            destinations: settings.backup_destinations,
            backup_targets: settings
                .backup_destinations
                .iter()
                .map(|destination| destination.join(target))
                .collect(),
        }
    }
}

/// The error for a target without a result. Never expected: the verified copy gives
/// one result per target; reported instead of panicking.
fn unfinished(target: &Path) -> CopyError {
    CopyError {
        step: CopyStep::Verify,
        kind: CopyErrorKind::Io {
            path: target.to_path_buf(),
            error: io::Error::other("the copy did not finish"),
        },
        cleanup: None,
    }
}

fn failed(error: FileError) -> Imported {
    (FileOutcome::Failed(error), Vec::new())
}
