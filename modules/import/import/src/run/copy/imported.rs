// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What one file of an import run gets.

use std::path::PathBuf;

use domain::Checksum;
use history::SidecarRecord;

use crate::backup::BackupReport;
use crate::report::{CarriedSidecar, ExistingValue, FileOutcome};
use crate::scan::FoundFile;

/// What one file gets: its outcome at the import destination and at each backup
/// destination, the checksum of the source when the run read it, and its sidecar
/// (IMPORT-019).
pub(super) struct Imported {
    pub(super) outcome: FileOutcome,
    pub(super) backups: Vec<BackupReport>,
    pub(super) checksum: Option<Checksum>,
    /// The sidecar written at the import target, or in a dry run the one that would be.
    pub(super) sidecar: Option<String>,
    /// The sidecar written next to the copies that are verified, for the history.
    pub(super) sidecar_record: Option<SidecarRecord>,
    /// The backup destinations where a sidecar was there already and was kept.
    pub(super) sidecar_kept_at: Vec<PathBuf>,
    /// The source sidecars that came along, once the run wrote or named them.
    pub(super) carried: Vec<CarriedSidecar>,
    /// The values the carried-along sidecar had that differ from the import's.
    pub(super) existing_values: Vec<ExistingValue>,
}

impl Imported {
    pub(super) fn without_backups(outcome: FileOutcome) -> Self {
        Imported::with_backups(outcome, Vec::new(), None)
    }

    pub(super) fn with_backups(
        outcome: FileOutcome,
        backups: Vec<BackupReport>,
        checksum: Option<Checksum>,
    ) -> Self {
        Imported {
            outcome,
            backups,
            checksum,
            sidecar: None,
            sidecar_record: None,
            sidecar_kept_at: Vec::new(),
            carried: Vec::new(),
            existing_values: Vec::new(),
        }
    }

    /// The source sidecars of `file` as the run left them; not written anywhere when
    /// the file got no sidecars.
    pub(super) fn carried_sidecars(&mut self, file: &FoundFile) -> Vec<CarriedSidecar> {
        if self.carried.len() == file.sidecars.len() {
            return std::mem::take(&mut self.carried);
        }
        file.sidecars
            .iter()
            .map(|sidecar| CarriedSidecar {
                path: sidecar.path.clone(),
                target: None,
                verified_at: 0,
            })
            .collect()
    }
}
