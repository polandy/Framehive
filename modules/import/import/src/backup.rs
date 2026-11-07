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
    /// It is the video destination (IMPORT-013).
    IsVideoDestination,
    /// It lies inside the video destination (IMPORT-013).
    InsideVideoDestination,
    /// It contains the video destination (IMPORT-013).
    ContainsVideoDestination,
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
            BackupDestinationProblem::IsVideoDestination => "it is the video destination",
            BackupDestinationProblem::InsideVideoDestination => {
                "it lies inside the video destination"
            }
            BackupDestinationProblem::ContainsVideoDestination => {
                "it contains the video destination"
            }
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
/// are absolute and without `..`. A `source` of `None` lies on a device of its own,
/// such as a camera (ADR 0034), so no backup destination can lie inside it.
///
/// # Errors
///
/// Returns a [`BackupDestinationError`] for the first backup destination that is
/// invalid.
pub fn check_backup_destinations(
    source: Option<&Path>,
    destination: &Path,
    backup_destinations: &[PathBuf],
) -> Result<(), BackupDestinationError> {
    for (index, backup) in backup_destinations.iter().enumerate() {
        let problem = match overlap(backup, destination) {
            Some(Overlap::Is) => Some(BackupDestinationProblem::IsImportDestination),
            Some(Overlap::Inside) => Some(BackupDestinationProblem::InsideImportDestination),
            Some(Overlap::Contains) => Some(BackupDestinationProblem::ContainsImportDestination),
            None if source.is_some_and(|source| backup.starts_with(source)) => {
                Some(BackupDestinationProblem::InsideSource)
            }
            None if backup_destinations[..index].contains(backup) => {
                Some(BackupDestinationProblem::GivenTwice)
            }
            None => None,
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

/// Checks backup destinations against the video destination: none may be it, lie
/// inside it or contain it (IMPORT-013, "Before the run"). The paths are compared as
/// given: all are absolute and without `..`.
///
/// # Errors
///
/// Returns a [`BackupDestinationError`] for the first backup destination that is
/// invalid.
pub fn check_against_video_destination(
    video_destination: &Path,
    backup_destinations: &[PathBuf],
) -> Result<(), BackupDestinationError> {
    for backup in backup_destinations {
        let problem = match overlap(backup, video_destination) {
            Some(Overlap::Is) => BackupDestinationProblem::IsVideoDestination,
            Some(Overlap::Inside) => BackupDestinationProblem::InsideVideoDestination,
            Some(Overlap::Contains) => BackupDestinationProblem::ContainsVideoDestination,
            None => continue,
        };
        return Err(BackupDestinationError {
            path: backup.clone(),
            problem,
        });
    }
    Ok(())
}

/// How a backup destination overlaps another destination.
enum Overlap {
    Is,
    Inside,
    Contains,
}

/// How `backup` overlaps `destination`, if it does.
fn overlap(backup: &Path, destination: &Path) -> Option<Overlap> {
    if backup == destination {
        Some(Overlap::Is)
    } else if backup.starts_with(destination) {
        Some(Overlap::Inside)
    } else if destination.starts_with(backup) {
        Some(Overlap::Contains)
    } else {
        None
    }
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
    /// Whether photos are backed up there; a video backup destination of its own
    /// receives only videos (IMPORT-013).
    pub receives_photos: bool,
}

impl BackupCounts {
    /// No files counted yet at `destination`, which receives photos when
    /// `receives_photos`.
    pub(crate) fn new(destination: &Path, receives_photos: bool) -> Self {
        BackupCounts {
            destination: destination.to_path_buf(),
            receives_photos,
            ..BackupCounts::default()
        }
    }

    /// Adds the outcomes at this destination in `reports`, the backups of one file.
    pub(crate) fn count<'r>(&mut self, reports: impl Iterator<Item = &'r BackupReport>) {
        for report in reports.filter(|report| report.destination == self.destination) {
            match report.outcome {
                BackupOutcome::BackedUp => self.backed_up += 1,
                BackupOutcome::AlreadyBackedUp => self.already_backed_up += 1,
                BackupOutcome::BackedUpBefore => self.backed_up_before += 1,
                BackupOutcome::Failed(_) => self.failed += 1,
            }
        }
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
