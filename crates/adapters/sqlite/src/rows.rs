// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writing and reading the rows of `runs` and `copies` (ADR 0031, section 3).

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use domain::Checksum;
use history::{CopyKey, CopyRecord, FoundCopy, LoggedRun, RecordedTime, RunId, RunLog, RunStart};
use rusqlite::{OptionalExtension, Row, params, types::Type};

use crate::database::Database;
use crate::text::{capture_time_text, parse_capture_time, parse_recorded_time};

/// The columns of a run with the number of its copies, as [`logged_run`] reads them.
const RUN_COLUMNS: &str = "SELECT runs.id, runs.started_at, runs.finished_at, runs.source, \
     runs.import_destination, count(copies.run_id) FROM runs \
     LEFT JOIN copies ON copies.run_id = runs.id";

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
            Ok(FoundCopy {
                target: row.get(0)?,
                checksum: checksum(row, 1)?,
            })
        },
    )?;
    rows.collect()
}

/// Every run, oldest first, with the number of its copies (IMPORT-010).
pub(crate) fn list_runs(database: &Database) -> rusqlite::Result<Vec<LoggedRun>> {
    let mut statement = database
        .connection
        .prepare(&format!("{RUN_COLUMNS} GROUP BY runs.id ORDER BY runs.id"))?;
    let runs = statement.query_map([], logged_run)?;
    runs.collect()
}

/// The log of `run`, or `None` when there is no such run (IMPORT-010).
pub(crate) fn read_run_log(database: &Database, run: RunId) -> rusqlite::Result<Option<RunLog>> {
    let logged = database
        .connection
        .query_row(
            &format!("{RUN_COLUMNS} WHERE runs.id = ?1 GROUP BY runs.id"),
            params![run.number()],
            logged_run,
        )
        .optional()?;
    let Some(logged) = logged else {
        return Ok(None);
    };
    let copies = run_copies(database, run)?;
    Ok(Some(RunLog::new(logged, copies)))
}

/// The copies of `run`, in the order they were recorded.
fn run_copies(database: &Database, run: RunId) -> rusqlite::Result<Vec<CopyRecord>> {
    let mut statement = database.connection.prepare(
        "SELECT destination, source_path, original_file_name, size, capture_time, \
         checksum, target, recorded_at FROM copies WHERE run_id = ?1 ORDER BY rowid",
    )?;
    let copies = statement.query_map(params![run.number()], |row| {
        let capture_time: Option<String> = row.get(4)?;
        Ok(CopyRecord {
            run,
            destination: path(row, 0)?,
            source_path: path(row, 1)?,
            original_file_name: row.get(2)?,
            size: size(row, 3)?,
            capture_time: capture_time
                .map(|text| parse_capture_time(&text))
                .transpose()
                .map_err(|error| conversion_error(4, error))?,
            checksum: checksum(row, 5)?,
            target: row.get(6)?,
            recorded_at: recorded_time(row, 7)?,
        })
    })?;
    copies.collect()
}

/// A row of [`RUN_COLUMNS`].
fn logged_run(row: &Row<'_>) -> rusqlite::Result<LoggedRun> {
    let finished_at: Option<String> = row.get(2)?;
    Ok(LoggedRun {
        id: RunId::new(row.get(0)?),
        start: RunStart {
            started_at: recorded_time(row, 1)?,
            source: path(row, 3)?,
            import_destination: path(row, 4)?,
        },
        finished_at: finished_at
            .map(|text| parse_recorded_time(&text))
            .transpose()
            .map_err(|error| conversion_error(2, error))?,
        copy_count: size(row, 5)?,
    })
}

/// A path column, from its bytes.
fn path(row: &Row<'_>, column: usize) -> rusqlite::Result<PathBuf> {
    let bytes: Vec<u8> = row.get(column)?;
    Ok(PathBuf::from(OsStr::from_bytes(&bytes)))
}

/// A size or count column, which is never negative.
fn size(row: &Row<'_>, column: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(column)?;
    u64::try_from(value).map_err(|error| conversion_error(column, error))
}

fn checksum(row: &Row<'_>, column: usize) -> rusqlite::Result<Checksum> {
    let text: String = row.get(column)?;
    text.parse::<Checksum>()
        .map_err(|error| conversion_error(column, error))
}

fn recorded_time(row: &Row<'_>, column: usize) -> rusqlite::Result<RecordedTime> {
    let text: String = row.get(column)?;
    parse_recorded_time(&text).map_err(|error| conversion_error(column, error))
}

/// The error for a column whose value does not have the format of ADR 0031,
/// section 3.
fn conversion_error(
    column: usize,
    error: impl std::error::Error + Send + Sync + 'static,
) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
}

/// A path as its bytes, so that a path that is not valid UTF-8 is kept exactly.
fn path_bytes(path: &Path) -> &[u8] {
    OsStr::as_bytes(path.as_os_str())
}

/// A size as SQLite's signed integer.
fn size_value(size: u64) -> rusqlite::Result<i64> {
    i64::try_from(size).map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}
