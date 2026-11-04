// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `sqlite` adapter of Framehive: it implements the `ImportHistory` port of
//! `history` with a SQLite database in the state folder (ADR 0031). Only apps depend
//! on it. See `README.md`.

#![forbid(unsafe_code)]

mod database;
mod rows;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use history::{
    CopyKey, CopyRecord, FoundCopy, HistoryError, ImportHistory, RecordedTime, RunId, RunStart,
};

use database::{Access, Database};

/// The name of the database file in the state folder (ADR 0031, section 2).
pub const HISTORY_FILE_NAME: &str = "import-history.sqlite";

/// The `history::ImportHistory` port in `import-history.sqlite` of a state folder.
///
/// The database is opened on first use. A missing database reads as empty and is
/// created, with its folder, by the first write; a dry run therefore creates nothing.
/// A database that is broken or has another layout is never changed (ADR 0031,
/// section 6).
#[derive(Debug)]
pub struct SqliteImportHistory {
    file: PathBuf,
    database: Mutex<Option<Database>>,
}

impl SqliteImportHistory {
    /// The history in `folder`, the state folder; nothing is read or written yet.
    #[must_use]
    pub fn new(folder: &Path) -> Self {
        SqliteImportHistory {
            file: folder.join(HISTORY_FILE_NAME),
            database: Mutex::new(None),
        }
    }

    /// The database file.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Runs `work` on the open database, opening it first when needed. For
    /// [`Access::Read`], `None` when there is no history yet.
    fn with_database<T>(
        &self,
        access: Access,
        work: impl FnOnce(&Database) -> rusqlite::Result<T>,
    ) -> Result<Option<T>, HistoryError> {
        let mut guard = self.database();
        if guard.is_none() {
            *guard = Database::open(&self.file, access).map_err(|error| self.error(&error))?;
        }
        let Some(database) = guard.as_mut() else {
            return Ok(None);
        };
        if access == Access::Write {
            database
                .create_schema()
                .map_err(|error| self.error(&error))?;
        } else if !database.has_schema() {
            return Ok(None);
        }
        work(database).map(Some).map_err(|error| self.error(&error))
    }

    fn database(&self) -> MutexGuard<'_, Option<Database>> {
        // A panic while holding the lock leaves a usable connection or none.
        self.database.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn error(&self, error: &dyn std::fmt::Display) -> HistoryError {
        HistoryError {
            file: self.file.clone(),
            reason: error.to_string(),
        }
    }
}

impl ImportHistory for SqliteImportHistory {
    fn start_run(&self, run: &RunStart) -> Result<RunId, HistoryError> {
        let id = self.with_database(Access::Write, |database| rows::insert_run(database, run))?;
        id.ok_or_else(|| self.error(&"the database was not opened for writing"))
    }

    fn finish_run(&self, run: RunId, finished_at: RecordedTime) -> Result<(), HistoryError> {
        self.with_database(Access::Write, |database| {
            rows::finish_run(database, run, finished_at)
        })?;
        Ok(())
    }

    fn record_copy(&self, copy: &CopyRecord) -> Result<(), HistoryError> {
        self.with_database(Access::Write, |database| rows::insert_copy(database, copy))?;
        Ok(())
    }

    fn find_copies(&self, key: &CopyKey<'_>) -> Result<Vec<FoundCopy>, HistoryError> {
        let found =
            self.with_database(Access::Read, |database| rows::find_copies(database, key))?;
        Ok(found.unwrap_or_default())
    }
}
