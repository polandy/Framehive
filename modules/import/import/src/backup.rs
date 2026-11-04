// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Backup destinations: which are valid, what a file found at a backup target means,
//! and what the import reports for each (IMPORT-007, ADR 0030).

use std::fmt;
use std::path::{Path, PathBuf};

use files::{CopyError, EntryKind, FileSystem};

use crate::conflicts::ConflictError;

/// Why a backup destination keeps the import run from starting (IMPORT-007.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackupDestinationProblem {
    /// It is the import destination.
    IsImportDestination,
    /// It lies inside the import destination.
    InsideImportDestination,
    /// It contains the import destination.
    ContainsImportDestination,
    /// It lies inside the source, or is the source.
    InsideSource,
    /// The same backup destination is given more than once.
    GivenTwice,
}

impl fmt::Display for BackupDestinationProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            BackupDestinationProblem::IsImportDestination => "it is the import destination",
            BackupDestinationProblem::InsideImportDestination => {
                "it lies inside the import destination"
            }
            BackupDestinationProblem::ContainsImportDestination => {
                "it contains the import destination"
            }
            BackupDestinationProblem::InsideSource => "it lies inside the source",
            BackupDestinationProblem::GivenTwice => "it is given twice",
        })
    }
}

/// A backup destination that keeps the import run from starting; nothing is written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid backup destination {}: {problem}", path.display())]
pub struct BackupDestinationError {
    /// The backup destination, as given.
    pub path: PathBuf,
    /// What is wrong with it.
    pub problem: BackupDestinationProblem,
}

/// Checks the backup destinations against the source, the import destination and
/// each other (IMPORT-007, "Before the run"). The paths are compared as given: all
/// are absolute and without `..`.
///
/// # Errors
///
/// Returns a [`BackupDestinationError`] for the first backup destination that is
/// invalid.
pub fn check_backup_destinations(
    source: &Path,
    destination: &Path,
    backup_destinations: &[PathBuf],
) -> Result<(), BackupDestinationError> {
    for (index, backup) in backup_destinations.iter().enumerate() {
        let problem = if backup == destination {
            Some(BackupDestinationProblem::IsImportDestination)
        } else if backup.starts_with(destination) {
            Some(BackupDestinationProblem::InsideImportDestination)
        } else if destination.starts_with(backup) {
            Some(BackupDestinationProblem::ContainsImportDestination)
        } else if backup.starts_with(source) {
            Some(BackupDestinationProblem::InsideSource)
        } else if backup_destinations[..index].contains(backup) {
            Some(BackupDestinationProblem::GivenTwice)
        } else {
            None
        };
        if let Some(problem) = problem {
            return Err(BackupDestinationError {
                path: backup.clone(),
                problem,
            });
        }
    }
    Ok(())
}

/// What happened to one file at one backup destination.
#[derive(Debug)]
pub struct BackupReport {
    /// The backup destination, as given in the settings.
    pub destination: PathBuf,
    /// What happened there.
    pub outcome: BackupOutcome,
}

/// What happened to one file at one backup destination (IMPORT-007).
#[derive(Debug)]
pub enum BackupOutcome {
    /// Copied to the backup target and verified against the source.
    BackedUp,
    /// The backup target already holds the file's content; it was not written.
    AlreadyBackedUp,
    /// The import history holds a verified copy of the file at this backup
    /// destination; nothing was written, and the backup target is not looked at
    /// (IMPORT-009).
    BackedUpBefore,
    /// The backup failed; nothing of it is left at the backup target.
    Failed(BackupError),
}

/// Why a file was not backed up at one backup destination. The import goes on.
#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    /// The backup target holds a different file, or something that is not a file; a
    /// backup never overwrites.
    #[error("name conflict: {} holds a different file", path.display())]
    NameConflict {
        /// The backup target.
        path: PathBuf,
    },
    /// The backup target or the source could not be read to compare them.
    #[error(transparent)]
    CannotCheckTarget(#[from] ConflictError),
    /// A step of the verified copy failed (IMPORT-004).
    #[error(transparent)]
    Copy(#[from] CopyError),
}

/// How many files one backup destination got in an import run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackupCounts {
    /// The backup destination, as given in the settings.
    pub destination: PathBuf,
    /// Files copied there and verified.
    pub backed_up: usize,
    /// Files that were there already with the same content.
    pub already_backed_up: usize,
    /// Files the import history holds a verified copy of there (IMPORT-009).
    pub backed_up_before: usize,
    /// Files that failed there.
    pub failed: usize,
}

impl BackupCounts {
    /// Counts the outcomes at `destination` in `reports`, the backups of each file.
    pub(crate) fn of<'r>(
        destination: &Path,
        reports: impl Iterator<Item = &'r BackupReport>,
    ) -> Self {
        let mut counts = BackupCounts {
            destination: destination.to_path_buf(),
            ..BackupCounts::default()
        };
        for report in reports.filter(|report| report.destination == destination) {
            match report.outcome {
                BackupOutcome::BackedUp => counts.backed_up += 1,
                BackupOutcome::AlreadyBackedUp => counts.already_backed_up += 1,
                BackupOutcome::BackedUpBefore => counts.backed_up_before += 1,
                BackupOutcome::Failed(_) => counts.failed += 1,
            }
        }
        counts
    }
}

/// What a backup target holds before the copy.
pub(crate) enum BackupTarget {
    /// Nothing: the file is copied there.
    Free,
    /// A file of the source's size: compared by checksum once the source is read.
    SameSize,
    /// The import history holds the file there: nothing is written or looked at.
    BackedUpBefore,
    /// Decided without a copy: a conflict, or a target that cannot be checked.
    Failed(BackupError),
}

/// Inspects the backup target `path` for a source file of `size` bytes. A missing
/// folder on the way, even a missing backup destination, means the target is free.
pub(crate) fn inspect_backup_target(
    file_system: &dyn FileSystem,
    path: &Path,
    size: u64,
) -> BackupTarget {
    let cannot_check = |source| {
        BackupTarget::Failed(BackupError::CannotCheckTarget(ConflictError {
            path: path.to_path_buf(),
            source,
        }))
    };
    let conflict = || {
        BackupTarget::Failed(BackupError::NameConflict {
            path: path.to_path_buf(),
        })
    };
    match file_system.entry(path) {
        Ok(None) => BackupTarget::Free,
        Ok(Some(EntryKind::File)) => match file_system.file_size(path) {
            Ok(found) if found == size => BackupTarget::SameSize,
            Ok(_) => conflict(),
            Err(error) => cannot_check(error),
        },
        Ok(Some(_)) => conflict(),
        Err(error) => cannot_check(error),
    }
}
