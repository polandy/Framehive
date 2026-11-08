// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the report says about the source sidecars that came with a file (IMPORT-020).

use std::fmt;
use std::path::PathBuf;

/// A source sidecar that came with its file (IMPORT-020).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarriedSidecar {
    /// The source sidecar, relative to the source.
    pub path: PathBuf,
    /// Where it went, relative to the destination with `/` between segments; in a dry
    /// run where it would go. `None` when the file was not copied.
    pub target: Option<String>,
    /// The number of locations it was written and verified at (IMPORT-020.8).
    pub verified_at: usize,
}

/// A sidecar field in which a carried-along sidecar already had a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidecarField {
    /// The job code (`photoshop:TransmissionReference`).
    JobCode,
    /// The creators (`dc:creator`), joined with `; `.
    Creator,
    /// The copyright notice (`dc:rights`).
    Copyright,
}

impl fmt::Display for SidecarField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            SidecarField::JobCode => "job code",
            SidecarField::Creator => "creator",
            SidecarField::Copyright => "copyright",
        })
    }
}

/// What became of a value a carried-along sidecar already had (IMPORT-020.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistingValueResult {
    /// The import's value replaced it.
    Replaced,
    /// It was kept; the import's value was not written.
    Kept,
}

/// A value a carried-along sidecar already had that differs from the import's
/// (IMPORT-020.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingValue {
    /// The field.
    pub field: SidecarField,
    /// The value the sidecar had.
    pub existing: String,
    /// The import's value.
    pub import: String,
    /// Which of the two the written sidecar holds.
    pub result: ExistingValueResult,
}
