// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Executing one planned file of an import run: conflict resolution, the verified
//! copy and the backups (IMPORT-003, IMPORT-004, IMPORT-007).

use std::io;
use std::path::{Path, PathBuf};

use domain::Checksum;
use files::{CopyError, CopyErrorKind, CopyStep, checksum_file, copy_verified_to_all};
use history::HistoryError;

use super::recognition::Recognition;
use super::resolvers::Resolvers;
use super::{FilePlan, ImportRun, ImportSettings, RecordedRun, RunFile};
use crate::backup::{
    BackupError, BackupOutcome, BackupReport, BackupTarget, inspect_backup_target,
};
use crate::conflicts::{ConflictError, ConflictOutcome, TargetRequest, TargetResolver};
use crate::plan::PlannedFile;
use crate::report::{FileError, FileOutcome, FileReport};
use crate::scan::FoundFile;

/// What one file gets: its outcome at the import destination and at each backup
/// destination, and the checksum of the source when the run read it.
struct Imported {
    outcome: FileOutcome,
    backups: Vec<BackupReport>,
    checksum: Option<Checksum>,
}

impl Imported {
    fn without_backups(outcome: FileOutcome) -> Self {
        Imported {
            outcome,
            backups: Vec::new(),
            checksum: None,
        }
    }
}

impl ImportRun<'_> {
    /// Imports one file of a planned run, records its verified copies in the import
    /// history when the run is recorded, and reports what happened to it. The error
    /// says that the history could not record a copy.
    pub(super) fn execute_file(
        &self,
        settings: &ImportSettings<'_>,
        resolvers: &mut Resolvers<'_>,
        file: RunFile,
        recorded: Option<RecordedRun>,
    ) -> (FileReport, Result<(), HistoryError>) {
        let role = file.found.role;
        let resolver = resolvers.of(role);
        let recognition = file.recognition.as_ref();
        let imported = match file.planned {
            Ok(FilePlan::Named(planned)) => {
                self.import_planned(settings, resolver, &file.found, &planned, recognition)
            }
            Ok(FilePlan::ImportedBefore { target }) => {
                self.back_up_imported_before(settings, resolver, &file.found, target, recognition)
            }
            Err(error) => Imported::without_backups(FileOutcome::Failed(error)),
        };
        let report = FileReport {
            path: file.found.path,
            dated_by_modification_time: file.no_capture_time,
            outcome: imported.outcome,
            backups: imported.backups,
            required_locations: 1 + settings.backup_destinations_of(role).len(),
        };
        let recorded = match (recorded, recognition) {
            (Some(run), Some(recognition)) => self.record_copies(
                settings.destination_of(role),
                run,
                recognition,
                &report,
                imported.checksum.as_ref(),
            ),
            _ => Ok(()),
        };
        (report, recorded)
    }

    /// Backs up a file imported before, at the target of its recorded copy, to each
    /// backup destination it is not backed up before at (IMPORT-009, "Backed up
    /// before"). The import destination is not looked at.
    fn back_up_imported_before(
        &self,
        settings: &ImportSettings<'_>,
        resolver: &mut TargetResolver<'_>,
        file: &FoundFile,
        target: String,
        recognition: Option<&Recognition>,
    ) -> Imported {
        if settings.dry_run {
            return Imported::without_backups(FileOutcome::ImportedBefore { target });
        }
        let copy = FileCopy::new(settings, file, &target, recognition);
        let copied = self.copy_file(&copy, resolver, false);
        Imported {
            outcome: FileOutcome::ImportedBefore { target },
            backups: copied.backups,
            checksum: copied.source_checksum,
        }
    }

    /// Resolves the conflicts of one planned file, copies it and backs it up.
    fn import_planned(
        &self,
        settings: &ImportSettings<'_>,
        resolver: &mut TargetResolver<'_>,
        file: &FoundFile,
        planned: &PlannedFile,
        recognition: Option<&Recognition>,
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
                Imported::without_backups(FileOutcome::WouldCopy { target })
            }
            ConflictOutcome::CopyTo { target } => {
                let copy = FileCopy::new(settings, file, &target, recognition);
                let copied = self.copy_file(&copy, resolver, true);
                let outcome = match copied.imported {
                    Some(Ok(checksum)) => FileOutcome::Copied { target, checksum },
                    Some(Err(error)) => FileOutcome::Failed(FileError::Copy(error)),
                    None => FileOutcome::Failed(FileError::Copy(unfinished(&copy.import_target))),
                };
                Imported {
                    outcome,
                    backups: copied.backups,
                    checksum: copied.source_checksum,
                }
            }
            // A file already imported is backed up, so that a re-run fills a new or a
            // failed backup destination (IMPORT-007).
            ConflictOutcome::AlreadyImported { target } if !settings.dry_run => {
                let copy = FileCopy::new(settings, file, &target, recognition);
                let copied = self.copy_file(&copy, resolver, false);
                // The resolver read the source to find it already imported, so its
                // checksum is known without another read.
                let checksum = copied
                    .source_checksum
                    .or_else(|| resolver.source_checksum(&file.path).ok());
                Imported {
                    outcome: FileOutcome::AlreadyImported { target },
                    backups: copied.backups,
                    checksum,
                }
            }
            ConflictOutcome::AlreadyImported { target } => {
                Imported::without_backups(FileOutcome::AlreadyImported { target })
            }
            ConflictOutcome::DuplicateOf { earlier } => {
                Imported::without_backups(FileOutcome::DuplicateOf { earlier })
            }
            ConflictOutcome::SkippedNameConflict { target } => {
                Imported::without_backups(FileOutcome::SkippedNameConflict { target })
            }
            ConflictOutcome::FailedNameConflict { target } => {
                failed(FileError::NameConflict { target })
            }
        }
    }

    /// Copies the file with one read of the source: to the import destination when
    /// `to_import_destination`, and to each backup target that is free and not backed
    /// up before.
    fn copy_file(
        &self,
        copy: &FileCopy<'_>,
        resolver: &mut TargetResolver<'_>,
        to_import_destination: bool,
    ) -> FileCopied {
        let found: Vec<BackupTarget> = copy
            .destinations
            .iter()
            .zip(&copy.backup_targets)
            .map(|(destination, path)| {
                if copy.is_backed_up_before(destination) {
                    BackupTarget::BackedUpBefore
                } else {
                    inspect_backup_target(self.file_system, path, copy.size)
                }
            })
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
                    BackupTarget::BackedUpBefore => BackupOutcome::BackedUpBefore,
                    BackupTarget::Failed(error) => BackupOutcome::Failed(error),
                };
                BackupReport {
                    destination: destination.clone(),
                    outcome,
                }
            })
            .collect();
        FileCopied {
            imported,
            backups,
            source_checksum,
        }
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

/// What [`ImportRun::copy_file`] did: the result at the import destination, if it was
/// a target, one report per backup destination, and the source's checksum if it was
/// read.
struct FileCopied {
    imported: Option<Result<Checksum, CopyError>>,
    backups: Vec<BackupReport>,
    source_checksum: Option<Checksum>,
}

/// The paths of one file's copy: its source, its import target and its backup
/// targets, all for the same target path below the file's own destinations
/// (IMPORT-013).
struct FileCopy<'a> {
    source_path: &'a Path,
    source: PathBuf,
    size: u64,
    import_target: PathBuf,
    destinations: &'a [PathBuf],
    backup_targets: Vec<PathBuf>,
    recognition: Option<&'a Recognition>,
}

impl<'a> FileCopy<'a> {
    fn new(
        settings: &ImportSettings<'a>,
        file: &'a FoundFile,
        target: &str,
        recognition: Option<&'a Recognition>,
    ) -> Self {
        let destinations = settings.backup_destinations_of(file.role);
        FileCopy {
            source_path: &file.path,
            source: settings.source.join(&file.path),
            size: file.size,
            import_target: settings.destination_of(file.role).join(target),
            destinations,
            backup_targets: destinations
                .iter()
                .map(|destination| destination.join(target))
                .collect(),
            recognition,
        }
    }

    /// Whether the import history holds the file at the backup destination
    /// `destination` (IMPORT-009, "Backed up before").
    fn is_backed_up_before(&self, destination: &Path) -> bool {
        self.recognition
            .is_some_and(|recognition| recognition.is_backed_up_before(destination))
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
    Imported::without_backups(FileOutcome::Failed(error))
}
