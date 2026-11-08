// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A file that gets an XMP sidecar takes a target only when the sidecar's name is
//! free as well, and claims both (IMPORT-019, "Conflicts").

use std::path::Path;

use metadata::xmp::SidecarNaming;

use super::{ConflictError, TargetRequest, TargetResolver, caseless_key};

impl TargetResolver<'_> {
    /// Whether the sidecar `file` would get at `candidate` is free: no earlier file
    /// or sidecar of the run claimed its name, no other job reserved it, and nothing
    /// is there at the destination. A leftover sidecar is never merged with
    /// (IMPORT-019.11). Always free for a file that gets no sidecar.
    pub(super) fn sidecar_is_free(
        &mut self,
        file: &TargetRequest<'_>,
        candidate: &str,
    ) -> Result<bool, ConflictError> {
        let Some(naming) = file.sidecar else {
            return Ok(true);
        };
        let sidecar = sidecar_target(naming, candidate);
        let key = caseless_key(&sidecar);
        if self.claimed.contains_key(&key)
            || self.claimed_sidecars.contains(&key)
            || self.needed_folders.contains(&key)
            || self.reserved_by_another_job(&sidecar)
        {
            return Ok(false);
        }
        Ok(self.existing(&sidecar)?.is_empty())
    }

    /// Claims the name of the sidecar `file` gets at `candidate`, if it gets one.
    pub(super) fn claim_sidecar(&mut self, file: &TargetRequest<'_>, candidate: &str) {
        if let Some(naming) = file.sidecar {
            self.claimed_sidecars
                .insert(caseless_key(&sidecar_target(naming, candidate)));
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
