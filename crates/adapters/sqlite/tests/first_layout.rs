// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A database of the first layout, from before runs had a video destination (ADR 0031,
//! addendum of 2026-11-06): writing migrates it to the current layout, one step per
//! layout, the import log reads it as it is.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use adapter_sqlite::{HISTORY_FILE_NAME, SqliteImportHistory};
use domain::{Timestamp, UtcOffset};
use history::{ImportHistory, LoggedRun, RecordedTime, RunId, RunStart};
use rusqlite::{Connection, TransactionBehavior};
use support::TempDir;

/// The schema of the first layout, exactly as the first version created it.
const FIRST_LAYOUT: &str = "
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

/// The time every row of these tests records, as the history writes it.
const RECORDED: &str = "2026-10-31T11:00:00+02:00";

fn now() -> RecordedTime {
    RecordedTime::at(
        Timestamp::from_unix_seconds(1_793_437_200),
        UtcOffset::from_seconds(7_200).unwrap(),
    )
    .unwrap()
}

/// Creates a database of the first layout in `folder` the way the first version did,
/// with one finished run that copied `DSCF0001.RAF` to `/library`.
fn create_first_layout(folder: &Path) -> PathBuf {
    let file = folder.join(HISTORY_FILE_NAME);
    let mut connection = Connection::open(&file).unwrap();
    connection
        .pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))
        .unwrap();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    transaction.execute_batch(FIRST_LAYOUT).unwrap();
    transaction.commit().unwrap();
    connection
        .execute(
            "INSERT INTO runs (started_at, source, import_destination) VALUES (?1, ?2, ?3)",
            (RECORDED, b"/media/card".as_slice(), b"/library".as_slice()),
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO copies (run_id, destination, source_path, original_file_name, size, \
             capture_time, checksum, target, recorded_at) \
             VALUES (1, ?1, ?2, 'DSCF0001.RAF', 10, NULL, 'blake3:abab', \
             '2026-10-17/DSCF0001.RAF', ?3)",
            (
                b"/library".as_slice(),
                b"DCIM/100_FUJI/DSCF0001.RAF".as_slice(),
                RECORDED,
            ),
        )
        .unwrap();
    connection
        .execute("UPDATE runs SET finished_at = ?1 WHERE id = 1", [RECORDED])
        .unwrap();
    file
}

/// The run [`create_first_layout`] recorded, as the import log lists it.
fn first_run() -> LoggedRun {
    LoggedRun {
        id: RunId::new(1),
        start: RunStart {
            source: PathBuf::from("/media/card"),
            import_destination: PathBuf::from("/library"),
            video_destination: None,
            started_at: now(),
        },
        finished_at: Some(now()),
        copy_count: 1,
    }
}

fn user_version(file: &Path) -> i64 {
    Connection::open(file)
        .unwrap()
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap()
}

/// spec: IMPORT-010.9
/// spec: IMPORT-013.10
/// spec: IMPORT-019.15
#[test]
fn writing_migrates_the_first_layout_and_keeps_its_runs() {
    let folder = TempDir::new();
    let file = create_first_layout(folder.path());
    let history = SqliteImportHistory::new(folder.path());
    let with_video = RunStart {
        video_destination: Some(PathBuf::from("/videos")),
        ..first_run().start
    };

    let run = history.start_run(&with_video).unwrap();
    let runs = history.list_runs().unwrap();
    let log = history.read_run_log(RunId::new(1)).unwrap().unwrap();
    drop(history);

    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0], first_run());
    assert_eq!(runs[1].id, run);
    assert_eq!(runs[1].start, with_video);
    assert_eq!(log.run, first_run());
    assert_eq!(log.copies.len(), 1);
    assert_eq!(log.copies[0].target, "2026-10-17/DSCF0001.RAF");
    assert_eq!(log.copies[0].sidecar, None);
    assert_eq!(user_version(&file), 3);
    let connection = Connection::open(&file).unwrap();
    let index: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'copies_by_run'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index, "CREATE INDEX copies_by_run ON copies (run_id)");
}

/// spec: IMPORT-010.9
#[test]
fn the_import_log_reads_the_first_layout_without_changing_it() {
    let folder = TempDir::new();
    let file = create_first_layout(folder.path());
    let content = fs::read(&file).unwrap();
    let history = SqliteImportHistory::read_only(folder.path());

    let runs = history.list_runs().unwrap();
    let log = history.read_run_log(RunId::new(1)).unwrap().unwrap();
    drop(history);

    assert_eq!(runs, [first_run()]);
    assert_eq!(log.run, first_run());
    assert_eq!(log.copies.len(), 1);
    assert_eq!(fs::read(&file).unwrap(), content);
    assert_eq!(user_version(&file), 1);
}

/// A database that claims the first layout but lacks its copies cannot be migrated;
/// the failed migration leaves it as it was (ADR 0031, section 6).
/// spec: IMPORT-009.15
#[test]
fn a_migration_that_fails_changes_nothing() {
    let folder = TempDir::new();
    let file = folder.path().join(HISTORY_FILE_NAME);
    Connection::open(&file)
        .unwrap()
        .execute_batch(
            "CREATE TABLE runs (id INTEGER PRIMARY KEY, started_at TEXT NOT NULL, \
             finished_at TEXT, source BLOB NOT NULL, import_destination BLOB NOT NULL); \
             PRAGMA user_version = 1;",
        )
        .unwrap();
    let content = fs::read(&file).unwrap();
    let history = SqliteImportHistory::new(folder.path());

    let error = history.start_run(&first_run().start).unwrap_err();
    drop(history);

    assert_eq!(error.file, file);
    assert!(error.to_string().contains("copies"), "{error}");
    assert_eq!(fs::read(&file).unwrap(), content);
    assert_eq!(user_version(&file), 1);
}
