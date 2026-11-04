// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Opening the database, checking its layout and creating its schema (ADR 0031,
//! sections 3, 4 and 6).

use std::fs;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, TransactionBehavior};

/// The `user_version` of the layout of ADR 0031, section 3.
pub(crate) const SCHEMA_VERSION: i64 = 1;

/// How long a write waits for another process's write (ADR 0031, section 4).
pub(crate) const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// The tables and the index of ADR 0031, section 3. Folder paths are BLOBs.
const SCHEMA: &str = "
CREATE TABLE runs (
    id INTEGER PRIMARY KEY,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    source BLOB NOT NULL,
    import_destination BLOB NOT NULL
);
CREATE TABLE copies (
    run_id INTEGER NOT NULL REFERENCES runs (id),
    destination BLOB NOT NULL,
    source_path BLOB NOT NULL,
    original_file_name TEXT NOT NULL,
    size INTEGER NOT NULL,
    capture_time TEXT,
    checksum TEXT NOT NULL,
    target TEXT NOT NULL,
    recorded_at TEXT NOT NULL
);
CREATE INDEX copies_by_file ON copies (destination, original_file_name, size);
PRAGMA user_version = 1;
";

/// What a call needs the database for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Access {
    /// Reading: a missing database is not created.
    Read,
    /// Writing: a missing database and its folder are created.
    Write,
}

/// Why the database cannot be used, before SQLite is asked.
#[derive(Debug)]
pub(crate) enum OpenError {
    Sqlite(rusqlite::Error),
    Folder(std::io::Error),
    UnknownLayout(String),
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::Sqlite(error) => error.fmt(formatter),
            OpenError::Folder(error) => write!(formatter, "the folder cannot be created: {error}"),
            OpenError::UnknownLayout(reason) => write!(formatter, "unknown layout: {reason}"),
        }
    }
}

impl From<rusqlite::Error> for OpenError {
    fn from(error: rusqlite::Error) -> Self {
        OpenError::Sqlite(error)
    }
}

/// An open database with a known layout: Framehive's, or none yet.
#[derive(Debug)]
pub(crate) struct Database {
    pub(crate) connection: Connection,
    has_schema: bool,
}

impl Database {
    /// Opens `file` for `access`. `None` for [`Access::Read`] when the file does not
    /// exist. Nothing is written to a database with another layout.
    pub(crate) fn open(file: &Path, access: Access) -> Result<Option<Self>, OpenError> {
        let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        if access == Access::Write {
            if let Some(folder) = file.parent() {
                fs::create_dir_all(folder).map_err(OpenError::Folder)?;
            }
            flags |= OpenFlags::SQLITE_OPEN_CREATE;
        } else if !file.try_exists().map_err(OpenError::Folder)? {
            return Ok(None);
        }
        let connection = Connection::open_with_flags(file, flags)?;
        connection.busy_timeout(BUSY_TIMEOUT)?;
        let has_schema = check_layout(&connection)?;
        // Per connection, not stored in the file: a recorded row survives a power loss.
        connection.pragma_update(None, "synchronous", "FULL")?;
        Ok(Some(Database {
            connection,
            has_schema,
        }))
    }

    /// Whether the database has Framehive's schema.
    pub(crate) fn has_schema(&self) -> bool {
        self.has_schema
    }

    /// Creates the schema of a database that has none yet, in WAL mode. Another
    /// process may create it at the same time; the transaction makes one of them win.
    pub(crate) fn create_schema(&mut self) -> rusqlite::Result<()> {
        if self.has_schema {
            return Ok(());
        }
        // The journal mode is stored in the file, and cannot change inside a
        // transaction.
        self.connection
            .pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 =
            transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version == 0 {
            transaction.execute_batch(SCHEMA)?;
        }
        transaction.commit()?;
        self.has_schema = true;
        Ok(())
    }
}

/// Whether the database has Framehive's schema (`true`) or is empty (`false`);
/// another layout is an error. Reading the header also rejects a file that is not a
/// SQLite database.
fn check_layout(connection: &Connection) -> Result<bool, OpenError> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version == SCHEMA_VERSION {
        return Ok(true);
    }
    if version != 0 {
        return Err(OpenError::UnknownLayout(format!(
            "user_version is {version}, not {SCHEMA_VERSION}"
        )));
    }
    let objects: i64 =
        connection.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))?;
    if objects != 0 {
        return Err(OpenError::UnknownLayout(
            "user_version is 0, but the database holds tables".to_owned(),
        ));
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

    use super::*;
    use crate::support::TempDir;

    fn pragma(database: &Database, name: &str) -> i64 {
        database
            .connection
            .pragma_query_value(None, name, |row| row.get(0))
            .unwrap()
    }

    /// spec: IMPORT-009.1
    #[test]
    fn a_connection_waits_five_seconds_and_syncs_fully() {
        let folder = TempDir::new();
        let file = folder.path().join("import-history.sqlite");
        let mut database = Database::open(&file, Access::Write).unwrap().unwrap();
        database.create_schema().unwrap();

        assert_eq!(pragma(&database, "busy_timeout"), 5_000);
        // FULL is 2.
        assert_eq!(pragma(&database, "synchronous"), 2);
        let journal: String = database
            .connection
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        assert_eq!(journal, "wal");
        assert_eq!(pragma(&database, "user_version"), SCHEMA_VERSION);
    }
}
