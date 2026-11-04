// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The port to the import history (ADR 0031, section 7).

use std::path::PathBuf;

use thiserror::Error;

use crate::{CopyKey, CopyRecord, FoundCopy, RecordedTime, RunId, RunStart};

/// Why the import history cannot be used: it cannot be opened, read or written, or has
/// an unknown layout. The history is not changed (ADR 0031, section 6).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("the import history {} cannot be used: {reason}", file.display())]
pub struct HistoryError {
    /// Where the history is stored, such as the database file.
    pub file: PathBuf,
    /// What went wrong.
    pub reason: String,
}

/// Port to the import history of this machine (ADR 0031); implemented by
/// `adapter-sqlite`.
///
/// A missing history is empty. Every implementation passes
/// [`testing::check_import_history`](crate::testing::check_import_history).
pub trait ImportHistory {
    /// Records the start of a run and returns its id.
    ///
    /// # Errors
    ///
    /// Returns a [`HistoryError`] when the history cannot be written.
    fn start_run(&self, run: &RunStart) -> Result<RunId, HistoryError>;

    /// Records the end of the run `run`.
    ///
    /// # Errors
    ///
    /// Returns a [`HistoryError`] when the history cannot be written.
    fn finish_run(&self, run: RunId, finished_at: RecordedTime) -> Result<(), HistoryError>;

    /// Records one verified copy, durably, before it returns (ADR 0031, section 4).
    ///
    /// # Errors
    ///
    /// Returns a [`HistoryError`] when the history cannot be written; then the copy
    /// is not recorded.
    fn record_copy(&self, copy: &CopyRecord) -> Result<(), HistoryError>;

    /// The recorded copies that match `key` ([`CopyRecord::matches`]), oldest first.
    ///
    /// # Errors
    ///
    /// Returns a [`HistoryError`] when the history cannot be read.
    fn find_copies(&self, key: &CopyKey<'_>) -> Result<Vec<FoundCopy>, HistoryError>;
}
