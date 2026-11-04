// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writing and reading the rows of `runs` and `copies` (ADR 0031, section 3).

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use domain::{Checksum, LocalDateTime};
use history::{CopyKey, CopyRecord, FoundCopy, RecordedTime, RunId, RunStart};
use rusqlite::{params, types::Type};

use crate::database::Database;

/// Records the start of `run` and returns its id.
pub(crate) fn insert_run(database: &Database, run: &RunStart) -> rusqlite::Result<RunId> {
    database.connection.execute(
        "INSERT INTO runs (started_at, source, import_destination) VALUES (?1, ?2, ?3)",
        params![
            run.started_at.to_string(),
            path_bytes(&run.source),
            path_bytes(&run.import_destination),
        ],
    )?;
    Ok(RunId::new(database.connection.last_insert_rowid()))
}

/// Records the end of `run`.
pub(crate) fn finish_run(
    database: &Database,
    run: RunId,
    finished_at: RecordedTime,
) -> rusqlite::Result<()> {
    database.connection.execute(
        "UPDATE runs SET finished_at = ?1 WHERE id = ?2",
        params![finished_at.to_string(), run.number()],
    )?;
    Ok(())
}

/// Records `copy` in a transaction of its own: one statement outside a transaction.
pub(crate) fn insert_copy(database: &Database, copy: &CopyRecord) -> rusqlite::Result<()> {
    database.connection.execute(
        "INSERT INTO copies (run_id, destination, source_path, original_file_name, size, \
         capture_time, checksum, target, recorded_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            copy.run.number(),
            path_bytes(&copy.destination),
            path_bytes(&copy.source_path),
            copy.original_file_name,
            size_value(copy.size)?,
            copy.capture_time.map(capture_time_text),
            copy.checksum.to_string(),
            copy.target,
            copy.recorded_at.to_string(),
        ],
    )?;
    Ok(())
}

/// The copies that match `key`, oldest first, through the index on destination,
/// original file name and size.
pub(crate) fn find_copies(
    database: &Database,
    key: &CopyKey<'_>,
) -> rusqlite::Result<Vec<FoundCopy>> {
    let mut statement = database.connection.prepare(
        "SELECT target, checksum FROM copies \
         WHERE destination = ?1 AND original_file_name = ?2 AND size = ?3 \
         AND (?4 IS NULL OR capture_time = ?4) ORDER BY rowid",
    )?;
    let rows = statement.query_map(
        params![
            path_bytes(key.destination),
            key.original_file_name,
            size_value(key.size)?,
            key.capture_time.map(capture_time_text),
        ],
        |row| {
            let checksum: String = row.get(1)?;
            Ok(FoundCopy {
                target: row.get(0)?,
                checksum: checksum.parse::<Checksum>().map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(1, Type::Text, Box::new(error))
                })?,
            })
        },
    )?;
    rows.collect()
}

/// A path as its bytes, so that a path that is not valid UTF-8 is kept exactly.
fn path_bytes(path: &Path) -> &[u8] {
    OsStr::as_bytes(path.as_os_str())
}

/// A size as SQLite's signed integer.
fn size_value(size: u64) -> rusqlite::Result<i64> {
    i64::try_from(size).map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

/// A capture time as `YYYY-MM-DDTHH:MM:SS` (ADR 0031, section 3).
fn capture_time_text(time: LocalDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        time.year(),
        time.month(),
        time.day(),
        time.hour(),
        time.minute(),
        time.second()
    )
}
