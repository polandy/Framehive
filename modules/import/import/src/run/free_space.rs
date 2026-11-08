// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The free-space check before the first copy: the bytes the plan writes to each
//! destination, added up per file system (IMPORT-012, ADR 0033).

use std::path::{Path, PathBuf};

use files::FileSystemId;

use super::copy::Sidecars;
use super::resolvers::Resolvers;
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
        let destinations = settings.all_destinations();
        let bytes = self.bytes_to_write(settings, plan, &destinations);
        let mut uses: Vec<FileSystemUse> = Vec::new();
        let mut check = FreeSpaceCheck::default();
        for (destination, bytes) in destinations.into_iter().zip(bytes) {
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

    /// The bytes written to each of `destinations`, every destination of the run
    /// (IMPORT-012, "Bytes to write"). Each file counts at its own destinations
    /// (IMPORT-013).
    fn bytes_to_write(
        &self,
        settings: &ImportSettings<'_>,
        plan: &PlannedRun,
        destinations: &[&Path],
    ) -> Vec<u64> {
        let mut resolvers = Resolvers::new(self, settings);
        let sidecars = Sidecars::of_run(self, settings, plan.naming.import_time.year());
        let mut bytes = vec![0_u64; destinations.len()];
        for file in &plan.files {
            let role = file.found.role;
            let Some((target, to_import_destination)) =
                written_target(resolvers.of(role), file, settings)
            else {
                continue;
            };
            let size = file
                .found
                .size
                .saturating_add(sidecar_size(sidecars.as_ref(), file));
            if to_import_destination {
                add_bytes(
                    &mut bytes,
                    destinations,
                    settings.destination_of(role),
                    size,
                );
            }
            for destination in settings.backup_destinations_of(role) {
                if self.writes_backup(file, destination, &target) {
                    add_bytes(&mut bytes, destinations, destination, size);
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
fn written_target(
    resolver: &mut TargetResolver<'_>,
    file: &RunFile,
    settings: &ImportSettings<'_>,
) -> Option<(String, bool)> {
    let planned = match file.planned.as_ref().ok()? {
        FilePlan::Named(planned) => planned,
        FilePlan::ImportedBefore { target } => return Some((target.clone(), false)),
    };
    let sidecars = settings.sidecar_namings_of(&file.found);
    let request = TargetRequest {
        source_path: &file.found.path,
        size: file.found.size,
        target: planned.target.as_ref().ok()?,
        sidecars: &sidecars,
    };
    match resolver.resolve(&request).ok()? {
        ConflictOutcome::CopyTo { target } => Some((target, true)),
        ConflictOutcome::AlreadyImported { target } => Some((target, false)),
        ConflictOutcome::DuplicateOf { .. }
        | ConflictOutcome::SkippedNameConflict { .. }
        | ConflictOutcome::FailedNameConflict { .. }
        | ConflictOutcome::TargetFolderIsAFile { .. } => None,
    }
}

/// The bytes of the sidecar that comes with `file`: none for a file the history knows.
fn sidecar_size(sidecars: Option<&Sidecars<'_>>, file: &RunFile) -> u64 {
    let Ok(FilePlan::Named(planned)) = &file.planned else {
        return 0;
    };
    sidecars.map_or(0, |sidecars| sidecars.size_of(&file.found, planned))
}

/// Adds `size` to the bytes written to `destination`, one of `destinations`.
fn add_bytes(bytes: &mut [u64], destinations: &[&Path], destination: &Path, size: u64) {
    if let Some(index) = destinations.iter().position(|known| *known == destination)
        && let Some(written) = bytes.get_mut(index)
    {
        *written = written.saturating_add(size);
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
