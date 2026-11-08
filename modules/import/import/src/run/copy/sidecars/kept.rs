// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The source sidecars a re-run finds already at a location: they count where they hold
//! what the import history recorded for them, or their source's bytes (IMPORT-020.9).

use std::path::Path;

use domain::Checksum;
use files::checksum_file;
use history::SidecarRecord;

use super::{Placement, Sidecars};
use crate::backup::{BackupOutcome, BackupReport};
use crate::conflicts::sidecar_target;
use crate::report::{CarriedSidecar, FileOutcome};
use crate::run::copy::Imported;
use crate::run::settings::PlannedSidecar;
use crate::scan::FoundFile;

impl Sidecars<'_> {
    /// Counts the source sidecars of `file`, imported before, where they hold what
    /// `history` recorded for them (IMPORT-020.9); nothing is written.
    pub(in crate::run::copy) fn count_imported_before(
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
        let destination = self.settings.destination_of(file.role);
        let carried: Vec<CarriedSidecar> = self
            .settings
            .sidecars_of(file)
            .iter()
            .filter_map(|sidecar| {
                let source = sidecar.source?;
                let name = sidecar_target(sidecar.naming, target);
                let expected = self.expected_content(sidecar, &name, history);
                let verified_at =
                    self.count_saved_at(&name, &expected, destination, &imported.backups);
                Some(CarriedSidecar {
                    path: source.path.clone(),
                    target: Some(name),
                    verified_at,
                })
            })
            .collect();
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
    /// locations of `placement` where it holds one of `expected`.
    pub(super) fn kept(
        &self,
        carried: Option<CarriedSidecar>,
        expected: &[Checksum],
        placement: &Placement<'_>,
        backups: &[BackupReport],
    ) -> Option<CarriedSidecar> {
        let mut carried = carried?;
        let name = carried.target.as_deref().unwrap_or_default();
        carried.verified_at = self.count_saved_at(name, expected, placement.destination, backups);
        Some(carried)
    }

    /// At how many locations the sidecar `name` holds one of `expected`: the import
    /// `destination` and each backup that has the file.
    fn count_saved_at(
        &self,
        name: &str,
        expected: &[Checksum],
        destination: &Path,
        backups: &[BackupReport],
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
            .filter(|location| self.holds(&location.join(name), expected))
            .count()
    }

    /// Whether the sidecar at `path` has one of the checksums `expected`.
    pub(super) fn holds(&self, path: &Path, expected: &[Checksum]) -> bool {
        !expected.is_empty()
            && checksum_file(self.files.files, self.files.hasher, path)
                .is_ok_and(|checksum| expected.contains(&checksum))
    }
}
