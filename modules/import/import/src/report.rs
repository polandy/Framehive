// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What an import run reports: per file, and through a [`ProgressSink`] while it runs
//! (ADR 0020 §1).

use std::io;
use std::path::{Path, PathBuf};

use domain::Checksum;
use files::CopyError;
use metadata::NoCaptureTime;
use naming::RenderError;

use crate::conflicts::ConflictError;
use crate::scan::{LeftOut, LeftOutReason, ScanError, SourceScan};

/// Receives the progress of an [`ImportRun`](crate::ImportRun) on the caller's thread,
/// as each step happens.
pub trait ProgressSink {
    /// The source was scanned; the run now dates, names and copies `scan.files`.
    fn scan_finished(&mut self, scan: &SourceScan);
    /// The run starts to resolve and copy the file at `path`, relative to the source.
    fn file_started(&mut self, path: &Path);
    /// The file is done: copied, not copied, or failed.
    fn file_finished(&mut self, report: &FileReport);
}

/// A [`ProgressSink`] that ignores every step, for callers that only need the
/// [`ImportSummary`].
#[derive(Debug, Clone, Copy, Default)]
pub struct NoProgress;

impl ProgressSink for NoProgress {
    fn scan_finished(&mut self, _scan: &SourceScan) {}
    fn file_started(&mut self, _path: &Path) {}
    fn file_finished(&mut self, _report: &FileReport) {}
}

/// What happened to one file the scan found.
#[derive(Debug)]
pub struct FileReport {
    /// The file, relative to the source.
    pub path: PathBuf,
    /// Why the file has no capture time, when it is dated by its modification time
    /// (CORE-005); `None` when it is dated by its capture time or failed before it
    /// was dated.
    pub dated_by_modification_time: Option<NoCaptureTime>,
    /// What happened to it.
    pub outcome: FileOutcome,
}

/// What happened to one file.
#[derive(Debug)]
pub enum FileOutcome {
    /// Copied to `target` and verified (IMPORT-004).
    Copied {
        /// The path relative to the destination, with `/` between segments.
        target: String,
        /// The checksum of the content, of the source and of the target.
        checksum: Checksum,
    },
    /// A dry run: the file would be copied to `target`; nothing was written.
    WouldCopy {
        /// The path relative to the destination, with `/` between segments.
        target: String,
    },
    /// The destination already holds the content at `target`; not copied (IMPORT-003).
    AlreadyImported {
        /// The existing file, relative to the destination.
        target: String,
    },
    /// An earlier file of the run has the same target and content; not copied.
    DuplicateOf {
        /// The earlier file, relative to the source.
        earlier: PathBuf,
    },
    /// The target holds a different file and the conflict policy is `skip`.
    SkippedNameConflict {
        /// The planned target.
        target: String,
    },
    /// The file failed; nothing of it is left at the destination.
    Failed(FileError),
}

/// Why one file failed. The run goes on with the next file (IMPORT-004.11).
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    /// The file could not be read to find its capture time.
    #[error("cannot be read: {error}")]
    CannotBeRead {
        /// The error of the file system.
        #[source]
        error: io::Error,
    },
    /// The file name is not valid UTF-8, so no target name can be made from it.
    #[error("the file name is not valid UTF-8")]
    NameNotUtf8,
    /// The modification time is outside the years 0 to 9999 in local time.
    #[error("the modification time cannot be shown as a local date and time")]
    ModificationTimeOutOfRange,
    /// The name template gives no valid target for this file (IMPORT-001).
    #[error("cannot be named: {0}")]
    CannotBeNamed(#[source] RenderError),
    /// The target holds a different file and the conflict policy is `fail`
    /// (IMPORT-003).
    #[error("name conflict: {target} holds a different file")]
    NameConflict {
        /// The planned target.
        target: String,
    },
    /// The import plan has no entry for this file. Never expected: the plan has one
    /// entry for each file that could be dated; reported instead of panicking.
    #[error("the import plan has no target for this file")]
    NotPlanned,
    /// The target folder or a file to compare with could not be read.
    #[error(transparent)]
    CannotCheckTarget(#[from] ConflictError),
    /// A step of the verified copy failed (IMPORT-004).
    #[error(transparent)]
    Copy(#[from] CopyError),
}

/// Why an import run did not start. Nothing was copied.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The source cannot be scanned (IMPORT-005).
    #[error(transparent)]
    Scan(#[from] ScanError),
    /// The clock shows a time outside the years 0 to 9999 in local time.
    #[error("the current time cannot be shown as a local date and time")]
    ImportTimeOutOfRange,
}

/// The result of a finished import run, in scan order.
#[derive(Debug, Default)]
pub struct ImportSummary {
    /// What happened to each file the scan found.
    pub files: Vec<FileReport>,
    /// The files and folders the scan left out.
    pub left_out: Vec<LeftOut>,
}

impl ImportSummary {
    /// Whether a file failed or the scan could not read a file or folder (CLI-001,
    /// exit code 1).
    #[must_use]
    pub fn has_failures(&self) -> bool {
        let failed = |report: &FileReport| matches!(report.outcome, FileOutcome::Failed(_));
        self.files.iter().any(failed)
            || self
                .left_out
                .iter()
                .any(|left_out| matches!(left_out.reason, LeftOutReason::CannotBeRead(_)))
    }
}
