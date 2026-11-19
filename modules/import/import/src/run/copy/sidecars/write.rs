// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writing one prepared sidecar and reading it back (IMPORT-019, "Writing").

use std::io::{self, Read};
use std::path::Path;

use metadata::xmp::{FoundSidecar, SidecarError, SidecarErrorKind, SidecarStep, parse_xmp};

use super::{Imported, Prepared, Sidecars, sidecar_error};
use crate::report::{FileError, FileOutcome};
use crate::run::settings::PlannedSidecar;

impl Sidecars<'_> {
    /// Writes `prepared` at the import destination's `path` and returns at how many
    /// locations it was verified there: 1, or 0 with the failure in `imported`.
    pub(super) fn write_at_import(
        &self,
        sidecar: &PlannedSidecar<'_>,
        path: &Path,
        name: &str,
        prepared: &Prepared,
        imported: &mut Imported,
    ) -> usize {
        match self.write_verified(path, prepared) {
            Ok(()) => {
                if sidecar.holds_values {
                    imported.sidecar = Some(name.to_owned());
                }
                1
            }
            Err(error) => {
                imported.outcome = FileOutcome::Failed(FileError::Sidecar(error));
                0
            }
        }
    }

    /// Writes `prepared` new at `path`, through a `.part` file and a rename, and
    /// requires it to read back byte for byte and to parse to the same packet.
    pub(super) fn write_verified(
        &self,
        path: &Path,
        prepared: &Prepared,
    ) -> Result<(), SidecarError> {
        let content = prepared.content.as_slice();
        self.files.write(path, content, &FoundSidecar::nothing())?;
        let back = self.read_back(path).map_err(|source| {
            let step = SidecarStep::Read;
            sidecar_error(path, SidecarErrorKind::Io { step, source })
        })?;
        if back != content {
            return Err(sidecar_error(path, SidecarErrorKind::Differs));
        }
        let Some(model) = &prepared.model else {
            return Ok(());
        };
        let parsed = parse_xmp(path, &back, self.files.xml)
            .map_err(|error| sidecar_error(path, SidecarErrorKind::Unreadable(error.reason)))?;
        if parsed != *model {
            return Err(sidecar_error(path, SidecarErrorKind::Differs));
        }
        Ok(())
    }

    pub(super) fn read_back(&self, path: &Path) -> io::Result<Vec<u8>> {
        let mut content = Vec::new();
        self.files
            .files
            .open_read(path)?
            .read_to_end(&mut content)?;
        Ok(content)
    }
}
