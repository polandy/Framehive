// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The sidecars a re-run finds already at a location: copied to the backups that lack
//! them (IMPORT-007.15), and a source sidecar counts where it holds what the import
//! history recorded for it, or its source's bytes (IMPORT-020.9).

use std::path::{Path, PathBuf};

use domain::Checksum;
use files::checksum_file;
use history::SidecarRecord;

use super::{
    Placement, Prepared, SidecarError, SidecarErrorKind, SidecarStep, Sidecars, sidecar_error,
};
use crate::backup::{BackupError, BackupOutcome, BackupReport};
use crate::conflicts::sidecar_target;
use crate::report::{CarriedSidecar, FileOutcome};
use crate::run::copy::Imported;
use crate::run::settings::PlannedSidecar;
use crate::scan::FoundFile;

impl Sidecars<'_> {
    /// Keeps the sidecar found at the import target and completes it at the backups
    /// that lack it (IMPORT-007.15).
    pub(super) fn keep_found(
        &self,
        carried: Option<CarriedSidecar>,
        expected: &[Checksum],
        name: &str,
        placement: &Placement<'_>,
        recorded: bool,
        imported: &mut Imported,
    ) -> Option<CarriedSidecar> {
        let written = if placement.dry_run {
            Vec::new()
        } else {
            self.copy_to_backups(name, placement.destination, recorded, imported)
        };
        self.kept(
            carried,
            expected,
            placement.destination,
            &imported.backups,
            &written,
        )
    }

    /// Copies the sidecar `name` kept at the import `destination` to each backup that
    /// got the file and has none, so that each holds the same bytes (IMPORT-007.15);
    /// read at most once. Returns the backups it was written and verified at.
    pub(super) fn copy_to_backups(
        &self,
        name: &str,
        destination: &Path,
        recorded: bool,
        imported: &mut Imported,
    ) -> Vec<PathBuf> {
        let at_import = destination.join(name);
        let mut copy = None;
        let mut written = Vec::new();
        let mut kept_at = vec![destination.to_path_buf()];
        for backup in &mut imported.backups {
            if !matches!(
                backup.outcome,
                BackupOutcome::BackedUp | BackupOutcome::AlreadyBackedUp
            ) {
                continue;
            }
            let at_backup = backup.destination.join(name);
            match self.copy_where_missing(&at_import, &at_backup, &mut copy) {
                Ok(true) => written.push(backup.destination.clone()),
                Ok(false) => kept_at.push(backup.destination.clone()),
                Err(error) => {
                    backup.outcome = BackupOutcome::Failed(BackupError::Sidecar(error));
                }
            }
        }
        if let (true, Some(copy)) = (recorded, copy) {
            imported.sidecar_kept_at = kept_at;
            imported.sidecar_record = Some(SidecarRecord {
                target: name.to_owned(),
                checksum: copy.checksum,
            });
        }
        written
    }

    /// Writes the sidecar at `source` to `target` unless something is there; `copy`
    /// holds its content once read. Returns whether it was written.
    fn copy_where_missing(
        &self,
        source: &Path,
        target: &Path,
        copy: &mut Option<Prepared>,
    ) -> Result<bool, SidecarError> {
        if !self.needs_writing(target, true)? {
            return Ok(false);
        }
        let content = match copy.take() {
            Some(content) => content,
            None => self.copy_of(source)?,
        };
        let result = self.write_verified(target, &content);
        *copy = Some(content);
        result.map(|()| true)
    }

    /// The sidecar at `path`, to be copied byte for byte.
    fn copy_of(&self, path: &Path) -> Result<Prepared, SidecarError> {
        let content = self.read_back(path).map_err(|source| {
            let step = SidecarStep::Read;
            sidecar_error(path, SidecarErrorKind::Io { step, source })
        })?;
        let mut hash = self.files.hasher.start();
        hash.update(&content);
        Ok(Prepared {
            content,
            model: None,
            checksum: hash.finish(),
            existing_values: Vec::new(),
        })
    }

    /// Copies the sidecars of `file`, imported before, from its import target to the
    /// backups this run got it at and that lack them (IMPORT-007.15), and counts its
    /// source sidecars where they hold what `history` recorded for them (IMPORT-020.9).
    pub(in crate::run::copy) fn add_to_imported_before(
        &self,
        file: &FoundFile,
        history: &[SidecarRecord],
        imported: &mut Imported,
    ) {
        let FileOutcome::ImportedBefore { target } = &imported.outcome else {
            return;
        };
        if self.settings.dry_run {
            return;
        }
        let target = target.clone();
        let destination = self.settings.destination_of(file.role);
        let sidecars = self.settings.sidecars_of(file);
        let recorded = recorded_position(&sidecars);
        let mut carried = Vec::new();
        for (position, sidecar) in sidecars.iter().enumerate() {
            let name = sidecar_target(sidecar.naming, &target);
            let at_import = self.files.files.entry(&destination.join(&name));
            let written = if matches!(at_import, Ok(Some(_))) {
                self.copy_to_backups(&name, destination, position == recorded, imported)
            } else {
                Vec::new()
            };
            let Some(source) = sidecar.source else {
                continue;
            };
            let expected = self.expected_content(sidecar, &name, history);
            let verified_at =
                self.count_saved_at(&name, &expected, destination, &imported.backups, &written);
            carried.push(CarriedSidecar {
                path: source.path.clone(),
                target: Some(name),
                verified_at,
            });
        }
        imported.carried = carried;
    }

    /// The checksums a source sidecar `name` counts with where it is kept; none when
    /// nothing there is kept.
    pub(super) fn expected_where_kept(
        &self,
        sidecar: &PlannedSidecar<'_>,
        placement: &Placement<'_>,
        name: &str,
        history: &[SidecarRecord],
    ) -> Vec<Checksum> {
        if sidecar.source.is_none() || !placement.keep_existing {
            return Vec::new();
        }
        self.expected_content(sidecar, name, history)
    }

    /// The checksums a kept sidecar `name` counts with: each one `history` recorded
    /// for it, and the source's when it is copied byte for byte (IMPORT-020.9).
    fn expected_content(
        &self,
        sidecar: &PlannedSidecar<'_>,
        name: &str,
        history: &[SidecarRecord],
    ) -> Vec<Checksum> {
        let mut expected: Vec<Checksum> = history
            .iter()
            .filter(|record| record.target == name)
            .map(|record| record.checksum.clone())
            .collect();
        if let Some(source) = sidecar.source.filter(|_| !sidecar.holds_values) {
            let path = self.settings.source.join(&source.path);
            if let Ok(checksum) = checksum_file(self.source_files, self.files.hasher, &path) {
                expected.push(checksum);
            }
        }
        expected
    }

    /// The source sidecar `carried`, kept at the import target, with the number of
    /// locations below `destination` and `backups` where it was `written` or holds one
    /// of `expected`.
    pub(super) fn kept(
        &self,
        carried: Option<CarriedSidecar>,
        expected: &[Checksum],
        destination: &Path,
        backups: &[BackupReport],
        written: &[PathBuf],
    ) -> Option<CarriedSidecar> {
        let mut carried = carried?;
        let name = carried.target.as_deref().unwrap_or_default();
        carried.verified_at = self.count_saved_at(name, expected, destination, backups, written);
        Some(carried)
    }

    /// At how many locations the sidecar `name` was `written` or holds one of
    /// `expected`: the import `destination` and each backup that has the file.
    fn count_saved_at(
        &self,
        name: &str,
        expected: &[Checksum],
        destination: &Path,
        backups: &[BackupReport],
        written: &[PathBuf],
    ) -> usize {
        let backups = backups
            .iter()
            .filter(|backup| {
                matches!(
                    backup.outcome,
                    BackupOutcome::BackedUp
                        | BackupOutcome::AlreadyBackedUp
                        | BackupOutcome::BackedUpBefore
                )
            })
            .map(|backup| backup.destination.as_path());
        std::iter::once(destination)
            .chain(backups)
            .filter(|location| {
                written.iter().any(|written| written == location)
                    || self.holds(&location.join(name), expected)
            })
            .count()
    }

    /// Whether the sidecar at `path` has one of the checksums `expected`.
    pub(super) fn holds(&self, path: &Path, expected: &[Checksum]) -> bool {
        !expected.is_empty()
            && checksum_file(self.files.files, self.files.hasher, path)
                .is_ok_and(|checksum| expected.contains(&checksum))
    }
}

/// The position of the sidecar the history records with each copy: the one with the
/// values, else the first.
pub(super) fn recorded_position(sidecars: &[PlannedSidecar<'_>]) -> usize {
    sidecars
        .iter()
        .position(|sidecar| sidecar.holds_values)
        .unwrap_or(0)
}
