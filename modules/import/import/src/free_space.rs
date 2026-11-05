// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the free-space check before an import finds: the file systems that cannot
//! hold what the run writes to them, and the destinations whose free space is unknown
//! (IMPORT-012, ADR 0033).

use std::fmt;
use std::io;
use std::path::PathBuf;

/// A file system that cannot hold the bytes the import run writes to its
/// destinations (IMPORT-012, "Not enough space").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceShortage {
    /// Every destination on the file system, as given in the settings: the import
    /// destination first, then the backup destinations in their order.
    pub destinations: Vec<PathBuf>,
    /// The bytes the run writes to all of them together.
    pub bytes_to_write: u64,
    /// The bytes free on the file system.
    pub available_bytes: u64,
}

impl fmt::Display for SpaceShortage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("not enough space at ")?;
        for (index, destination) in self.destinations.iter().enumerate() {
            if index > 0 {
                formatter.write_str(", ")?;
            }
            write!(formatter, "{}", destination.display())?;
        }
        write!(
            formatter,
            ": needs {} bytes, {} bytes free",
            self.bytes_to_write, self.available_bytes
        )
    }
}

/// A destination whose free space cannot be read; the import goes on without
/// checking it (IMPORT-012, "Free space unknown").
#[derive(Debug, thiserror::Error)]
#[error("cannot tell the free space at {}: {error}", destination.display())]
pub struct FreeSpaceUnknown {
    /// The destination, as given in the settings.
    pub destination: PathBuf,
    /// Why its free space cannot be read.
    #[source]
    pub error: io::Error,
}

/// What the free-space check found for an import plan (IMPORT-012).
#[derive(Debug, Default)]
pub struct FreeSpaceCheck {
    /// Every file system that is short, in the order of its first destination in the
    /// settings. The run does not start when there is one.
    pub shortages: Vec<SpaceShortage>,
    /// The destinations whose free space cannot be read, in the order of the settings.
    pub unknown: Vec<FreeSpaceUnknown>,
}

/// The import does not start: a file system cannot hold what the run writes to it
/// (IMPORT-012). Nothing is created or copied at any destination.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}", lines(.shortages))]
pub struct NotEnoughSpace {
    /// Every file system that is short, at least one.
    pub shortages: Vec<SpaceShortage>,
}

/// One line per shortage.
fn lines(shortages: &[SpaceShortage]) -> String {
    shortages
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}
