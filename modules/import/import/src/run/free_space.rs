// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The free-space check before the first copy: the bytes the plan writes to each
//! destination, added up per file system (IMPORT-012, ADR 0033).

use std::path::{Path, PathBuf};

use files::FileSystemId;

use super::{FilePlan, ImportRun, ImportSettings, PlannedRun, RunFile};
use crate::backup::{BackupTarget, inspect_backup_target};
use crate::conflicts::{ConflictOutcome, TargetRequest, TargetResolver};
use crate::free_space::{FreeSpaceCheck, FreeSpaceUnknown, SpaceShortage};

/// The destinations of one file system and the bytes written to them.
struct FileSystemUse {
    file_system: FileSystemId,
    available_bytes: u64,
    destinations: Vec<PathBuf>,
    bytes_to_write: u64,
}

impl ImportRun<'_> {
    /// Checks that each file system can hold the bytes the plan writes to all
    /// destinations on it (IMPORT-012). It resolves the conflicts of each file as the
    /// run would, and looks at each backup target, but writes nothing. A dry run is
    /// counted as the import it previews.
    #[must_use]
    pub fn check_free_space(
        &self,
        settings: &ImportSettings<'_>,
        plan: &PlannedRun,
    ) -> FreeSpaceCheck {
        let bytes = self.bytes_to_write(settings, plan);
        let destinations = std::iter::once(settings.destination)
            .chain(settings.backup_destinations.iter().map(PathBuf::as_path));
        let mut uses: Vec<FileSystemUse> = Vec::new();
        let mut check = FreeSpaceCheck::default();
        for (destination, bytes) in destinations.zip(bytes) {
            match self.free_space.free_space(destination) {
                Ok(space) => add_use(
                    &mut uses,
                    space.file_system,
                    space.available_bytes,
                    destination,
                    bytes,
                ),
                Err(error) => check.unknown.push(FreeSpaceUnknown {
                    destination: destination.to_path_buf(),
                    error,
                }),
            }
        }
        check.shortages = uses
            .into_iter()
            .filter(|used| used.bytes_to_write > used.available_bytes)
            .map(|used| SpaceShortage {
                destinations: used.destinations,
                bytes_to_write: used.bytes_to_write,
                available_bytes: used.available_bytes,
            })
            .collect();
        check
    }

    /// The bytes written to the import destination, then to each backup destination
    /// in the order of the settings (IMPORT-012, "Bytes to write").
    fn bytes_to_write(&self, settings: &ImportSettings<'_>, plan: &PlannedRun) -> Vec<u64> {
        let mut resolver = TargetResolver::new(
            self.file_system,
            self.hasher,
            settings.source,
            settings.destination,
            settings.policy,
        );
        let mut bytes = vec![0_u64; 1 + settings.backup_destinations.len()];
        for file in &plan.files {
            let Some((target, to_import_destination)) = written_target(&mut resolver, file) else {
                continue;
            };
            let size = file.found.size;
            if to_import_destination {
                bytes[0] = bytes[0].saturating_add(size);
            }
            for (index, destination) in settings.backup_destinations.iter().enumerate() {
                if self.writes_backup(file, destination, &target) {
                    bytes[index + 1] = bytes[index + 1].saturating_add(size);
                }
            }
        }
        bytes
    }

    /// Whether the file is written to the backup destination `destination`: it is not
    /// backed up there before, and its backup target holds nothing yet.
    fn writes_backup(&self, file: &RunFile, destination: &Path, target: &str) -> bool {
        let backed_up_before = file
            .recognition
            .as_ref()
            .is_some_and(|recognition| recognition.is_backed_up_before(destination));
        !backed_up_before
            && matches!(
                inspect_backup_target(self.file_system, &destination.join(target), file.found.size),
                BackupTarget::Free
            )
    }
}

/// The target of a file that is backed up, and whether it is also copied to the
/// import destination; `None` for a file that is written nowhere.
fn written_target(resolver: &mut TargetResolver<'_>, file: &RunFile) -> Option<(String, bool)> {
    let planned = match file.planned.as_ref().ok()? {
        FilePlan::Named(planned) => planned,
        FilePlan::ImportedBefore { target } => return Some((target.clone(), false)),
    };
    let request = TargetRequest {
        source_path: &file.found.path,
        size: file.found.size,
        target: planned.target.as_ref().ok()?,
    };
    match resolver.resolve(&request).ok()? {
        ConflictOutcome::CopyTo { target } => Some((target, true)),
        ConflictOutcome::AlreadyImported { target } => Some((target, false)),
        ConflictOutcome::DuplicateOf { .. }
        | ConflictOutcome::SkippedNameConflict { .. }
        | ConflictOutcome::FailedNameConflict { .. } => None,
    }
}

/// Adds `bytes` at `destination` to the use of its file system.
fn add_use(
    uses: &mut Vec<FileSystemUse>,
    file_system: FileSystemId,
    available_bytes: u64,
    destination: &Path,
    bytes: u64,
) {
    if let Some(used) = uses.iter_mut().find(|used| used.file_system == file_system) {
        used.destinations.push(destination.to_path_buf());
        used.bytes_to_write = used.bytes_to_write.saturating_add(bytes);
    } else {
        uses.push(FileSystemUse {
            file_system,
            available_bytes,
            destinations: vec![destination.to_path_buf()],
            bytes_to_write: bytes,
        });
    }
}
