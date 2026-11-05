// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! [`ImportHistory`] in memory.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::{
    CopyKey, CopyRecord, FoundCopy, HistoryError, ImportHistory, LoggedRun, RecordedTime, RunId,
    RunLog, RunStart,
};

/// The name [`MemoryImportHistory`] gives in its errors.
const MEMORY_FILE: &str = "memory";

/// [`ImportHistory`] in memory: tests read what was recorded and make the history
/// fail.
///
/// ```
/// use domain::{Checksum, Timestamp, UtcOffset};
/// use history::testing::MemoryImportHistory;
/// use history::{CopyKey, CopyRecord, ImportHistory, RecordedTime, RunStart};
/// use std::path::{Path, PathBuf};
///
/// let now = RecordedTime::at(Timestamp::UNIX_EPOCH, UtcOffset::from_seconds(0)?).unwrap();
/// let history = MemoryImportHistory::new();
/// let run = history.start_run(&RunStart {
///     source: PathBuf::from("/media/card"),
///     import_destination: PathBuf::from("/library"),
///     started_at: now,
/// })?;
/// history.record_copy(&CopyRecord {
///     run,
///     destination: PathBuf::from("/library"),
///     source_path: PathBuf::from("DSCF0001.RAF"),
///     original_file_name: "DSCF0001.RAF".to_owned(),
///     size: 10,
///     capture_time: None,
///     checksum: Checksum::new("blake3", vec![1])?,
///     target: "DSCF0001.RAF".to_owned(),
///     recorded_at: now,
/// })?;
/// let key = CopyKey {
///     destination: Path::new("/library"),
///     original_file_name: "DSCF0001.RAF",
///     size: 10,
///     capture_time: None,
/// };
/// assert_eq!(history.find_copies(&key)?.len(), 1);
/// assert_eq!(history.copies().len(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Default)]
pub struct MemoryImportHistory {
    state: Mutex<State>,
}

/// One run [`MemoryImportHistory`] recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedRun {
    /// The id the run was given.
    pub id: RunId,
    /// How it started.
    pub start: RunStart,
    /// When it ended; `None` while it runs.
    pub finished_at: Option<RecordedTime>,
}

#[derive(Debug, Default)]
struct State {
    runs: Vec<RecordedRun>,
    copies: Vec<CopyRecord>,
    broken: bool,
    /// The number of further copies to record before recording fails, if it fails.
    records_before_failure: Option<usize>,
}

impl State {
    /// `run` as the import log lists it, with the number of its copies.
    fn logged(&self, run: &RecordedRun) -> LoggedRun {
        let copies = self.copies.iter().filter(|copy| copy.run == run.id).count();
        LoggedRun {
            id: run.id,
            start: run.start.clone(),
            finished_at: run.finished_at,
            copy_count: u64::try_from(copies).unwrap_or(u64::MAX),
        }
    }
}

impl MemoryImportHistory {
    /// An empty history, as on the first run.
    #[must_use]
    pub fn new() -> Self {
        MemoryImportHistory::default()
    }

    /// Makes every later call fail, as a history that cannot be used does. Nothing
    /// recorded so far changes.
    pub fn break_storage(&self) {
        self.state().broken = true;
    }

    /// Whether [`MemoryImportHistory::break_storage`] was called.
    #[must_use]
    pub fn is_broken(&self) -> bool {
        self.state().broken
    }

    /// Records `count` more copies, then fails on every later
    /// [`ImportHistory::record_copy`].
    pub fn fail_recording_after(&self, count: usize) {
        self.state().records_before_failure = Some(count);
    }

    /// The runs recorded, in the order they started.
    #[must_use]
    pub fn runs(&self) -> Vec<RecordedRun> {
        self.state().runs.clone()
    }

    /// The copies recorded, in the order they were recorded.
    #[must_use]
    pub fn copies(&self) -> Vec<CopyRecord> {
        self.state().copies.clone()
    }

    fn state(&self) -> MutexGuard<'_, State> {
        // A test that panicked while holding the lock leaves usable values.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The state, or the error of a broken history.
    fn usable(&self) -> Result<MutexGuard<'_, State>, HistoryError> {
        let state = self.state();
        if state.broken {
            return Err(error("the history is broken"));
        }
        Ok(state)
    }
}

impl ImportHistory for MemoryImportHistory {
    fn start_run(&self, run: &RunStart) -> Result<RunId, HistoryError> {
        let mut state = self.usable()?;
        let id = RunId::new(i64::try_from(state.runs.len()).unwrap_or(i64::MAX) + 1);
        state.runs.push(RecordedRun {
            id,
            start: run.clone(),
            finished_at: None,
        });
        Ok(id)
    }

    fn finish_run(&self, run: RunId, finished_at: RecordedTime) -> Result<(), HistoryError> {
        let mut state = self.usable()?;
        let recorded = state
            .runs
            .iter_mut()
            .find(|recorded| recorded.id == run)
            .ok_or_else(|| error("the run was not started"))?;
        recorded.finished_at = Some(finished_at);
        Ok(())
    }

    fn record_copy(&self, copy: &CopyRecord) -> Result<(), HistoryError> {
        let mut state = self.usable()?;
        match &mut state.records_before_failure {
            Some(0) => return Err(error("recording fails")),
            Some(count) => *count -= 1,
            None => {}
        }
        state.copies.push(copy.clone());
        Ok(())
    }

    fn find_copies(&self, key: &CopyKey<'_>) -> Result<Vec<FoundCopy>, HistoryError> {
        let state = self.usable()?;
        Ok(state
            .copies
            .iter()
            .filter(|copy| copy.matches(key))
            .map(|copy| FoundCopy {
                target: copy.target.clone(),
                checksum: copy.checksum.clone(),
            })
            .collect())
    }

    fn list_runs(&self) -> Result<Vec<LoggedRun>, HistoryError> {
        let state = self.usable()?;
        Ok(state.runs.iter().map(|run| state.logged(run)).collect())
    }

    fn read_run_log(&self, run: RunId) -> Result<Option<RunLog>, HistoryError> {
        let state = self.usable()?;
        let Some(recorded) = state.runs.iter().find(|recorded| recorded.id == run) else {
            return Ok(None);
        };
        let copies = state
            .copies
            .iter()
            .filter(|copy| copy.run == run)
            .cloned()
            .collect();
        Ok(Some(RunLog::new(state.logged(recorded), copies)))
    }
}

fn error(reason: &str) -> HistoryError {
    HistoryError {
        file: PathBuf::from(MEMORY_FILE),
        reason: reason.to_owned(),
    }
}
