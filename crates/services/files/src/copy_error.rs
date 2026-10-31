// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Why a verified copy failed (IMPORT-004).

use std::fmt;
use std::io;
use std::path::PathBuf;

use domain::Checksum;
use thiserror::Error;

/// The step of a verified copy that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CopyStep {
    /// Creating the missing folders of the target path.
    CreateFolders,
    /// Reading the source file or its modification time.
    ReadSource,
    /// Writing, timestamping or syncing the `.part` file.
    WritePart,
    /// Renaming the `.part` file to the target name.
    Rename,
    /// Syncing the target's folder after the rename.
    SyncFolder,
    /// Reading the target back.
    ReadBack,
    /// Comparing the target's checksum with the source's.
    Verify,
}

impl fmt::Display for CopyStep {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            CopyStep::CreateFolders => "create the target folders",
            CopyStep::ReadSource => "read the source",
            CopyStep::WritePart => "write the .part file",
            CopyStep::Rename => "rename the .part file to the target",
            CopyStep::SyncFolder => "sync the target folder",
            CopyStep::ReadBack => "read the target back",
            CopyStep::Verify => "verify the target",
        })
    }
}

/// The reason a step of a verified copy failed.
#[derive(Debug, Error)]
pub enum CopyErrorKind {
    /// The file system reported an error for `path`.
    #[error("{path}: {error}", path = .path.display())]
    Io {
        /// The file or folder the failed operation was on.
        path: PathBuf,
        /// The error of the file system.
        #[source]
        error: io::Error,
    },
    /// A file appeared at the target path while the copy was written; it is kept.
    #[error("{path} already exists", path = .path.display())]
    TargetExists {
        /// The target path.
        path: PathBuf,
    },
    /// The target read back does not have the source's checksum; it was removed.
    #[error("the target has the checksum {target_checksum}, the source {source_checksum}")]
    ChecksumMismatch {
        /// The checksum of the source, computed while it was copied.
        source_checksum: Checksum,
        /// The checksum of the target as it was read back.
        target_checksum: Checksum,
    },
}

/// A file that a failed verified copy wrote and could not remove.
#[derive(Debug, Error)]
#[error("{path} could not be removed: {error}", path = .path.display())]
pub struct CleanupFailure {
    /// The `.part` file or target that is left behind.
    pub path: PathBuf,
    /// Why it could not be removed.
    #[source]
    pub error: io::Error,
}

/// Error returned by [`copy_verified`](crate::copy_verified): the step that failed,
/// why, and a file that could not be cleaned up, if any.
#[derive(Debug, Error)]
#[error("cannot {step}: {kind}{}", cleanup_note(.cleanup.as_deref()))]
pub struct CopyError {
    /// The step that failed.
    pub step: CopyStep,
    /// Why it failed.
    #[source]
    pub kind: CopyErrorKind,
    /// A file the copy wrote and could not remove afterwards.
    pub cleanup: Option<Box<CleanupFailure>>,
}

fn cleanup_note(cleanup: Option<&CleanupFailure>) -> String {
    cleanup.map_or_else(String::new, |failure| format!("; and {failure}"))
}
