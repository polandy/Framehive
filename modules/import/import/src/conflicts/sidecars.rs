// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A file that gets an XMP sidecar takes a target only when the sidecar's name is
//! free as well, and claims both (IMPORT-019, "Conflicts").

use std::path::{Path, PathBuf};

use metadata::xmp::SidecarNaming;

use super::{ConflictError, TargetRequest, TargetResolver, caseless_key};

impl TargetResolver<'_> {
    /// Whether each sidecar `file` would get at `candidate` is free: no earlier file
    /// or sidecar of the run claimed its name, no other job reserved it, and nothing
    /// is there at the destination or at one of the file's backup destinations. A
    /// leftover sidecar is never merged with (IMPORT-019.11, IMPORT-007.14). Always
    /// free for a file that gets no sidecar.
    pub(super) fn sidecar_is_free(
        &mut self,
        file: &TargetRequest<'_>,
        candidate: &str,
    ) -> Result<bool, ConflictError> {
        for naming in file.sidecars {
            if !self.sidecar_name_is_free(*naming, candidate, file.backups)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn sidecar_name_is_free(
        &mut self,
        naming: SidecarNaming,
        candidate: &str,
        backups: &[PathBuf],
    ) -> Result<bool, ConflictError> {
        let sidecar = sidecar_target(naming, candidate);
        let key = caseless_key(&sidecar);
        if self.claimed.contains_key(&key)
            || self.claimed_sidecars.contains(&key)
            || self.needed_folders.contains(&key)
            || self.reserved_by_another_job(&sidecar)
        {
            return Ok(false);
        }
        Ok(self.existing(&sidecar)?.is_empty() && !self.taken_at_a_backup(&sidecar, backups))
    }

    /// Whether something is at `sidecar` below one of `backups`. One that cannot be
    /// checked counts as free: that backup reports the failure when it is written.
    fn taken_at_a_backup(&self, sidecar: &str, backups: &[PathBuf]) -> bool {
        backups.iter().any(|backup| {
            self.file_system
                .entry(&backup.join(sidecar))
                .is_ok_and(|entry| entry.is_some())
        })
    }

    /// Claims the name of each sidecar `file` gets at `candidate`.
    pub(super) fn claim_sidecar(&mut self, file: &TargetRequest<'_>, candidate: &str) {
        for naming in file.sidecars {
            self.claimed_sidecars
                .insert(caseless_key(&sidecar_target(*naming, candidate)));
        }
    }

    /// Whether an earlier file of the run claimed `candidate` for its sidecar.
    pub(super) fn claimed_for_a_sidecar(&self, candidate: &str) -> bool {
        self.claimed_sidecars.contains(&caseless_key(candidate))
    }
}

/// The sidecar of the file at `target`, relative to the destination with `/` between
/// segments.
pub(crate) fn sidecar_target(naming: SidecarNaming, target: &str) -> String {
    naming
        .sidecar_path(Path::new(target))
        .to_string_lossy()
        .into_owned()
}
