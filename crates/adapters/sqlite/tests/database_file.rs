// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The database file of ADR 0031: where it is, when it is created, and that a database
//! with another layout is never changed.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use adapter_sqlite::{HISTORY_FILE_NAME, SqliteImportHistory};
use domain::{Checksum, LocalDateTime, Timestamp, UtcOffset};
use history::{CopyKey, CopyRecord, ImportHistory, RecordedTime, RunStart};
use rusqlite::Connection;
use support::TempDir;

fn now() -> RecordedTime {
    RecordedTime::at(
        Timestamp::from_unix_seconds(1_793_437_200),
        UtcOffset::from_seconds(7_200).unwrap(),
    )
    .unwrap()
}

fn key(destination: &Path) -> CopyKey<'_> {
    CopyKey {
        destination,
        original_file_name: "DSCF0001.RAF",
        size: 10,
        capture_time: None,
    }
}

fn run_start() -> RunStart {
    RunStart {
        source: PathBuf::from("/media/card"),
        import_destination: PathBuf::from("/library"),
        started_at: now(),
    }
}

/// spec: IMPORT-009.14
#[test]
fn reading_a_missing_history_creates_nothing() {
    let folder = TempDir::new();
    let state_folder = folder.path().join("framehive");
    let history = SqliteImportHistory::new(&state_folder);

    let found = history.find_copies(&key(Path::new("/library"))).unwrap();

    assert!(found.is_empty());
    assert!(!state_folder.exists());
}

/// spec: IMPORT-009.1
#[test]
fn the_history_is_a_file_in_the_state_folder_with_every_column() {
    let folder = TempDir::new();
    let history = SqliteImportHistory::new(folder.path());
    let run = history.start_run(&run_start()).unwrap();
    history
        .record_copy(&CopyRecord {
            run,
            destination: PathBuf::from("/library"),
            source_path: PathBuf::from("DCIM/100_FUJI/DSCF0001.RAF"),
            original_file_name: "DSCF0001.RAF".to_owned(),
            size: 10,
            capture_time: Some(LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap()),
            checksum: Checksum::new(Checksum::BLAKE3, vec![0xab; 2]).unwrap(),
            target: "2026-10-17/DSCF0001.RAF".to_owned(),
            recorded_at: now(),
        })
        .unwrap();
    history.finish_run(run, now()).unwrap();
    drop(history);

    let connection = Connection::open(folder.path().join(HISTORY_FILE_NAME)).unwrap();
    let copy: (
        Vec<u8>,
        Vec<u8>,
        String,
        i64,
        String,
        String,
        String,
        String,
    ) = connection
        .query_row(
            "SELECT destination, source_path, original_file_name, size, capture_time, \
             checksum, target, recorded_at FROM copies",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        copy,
        (
            b"/library".to_vec(),
            b"DCIM/100_FUJI/DSCF0001.RAF".to_vec(),
            "DSCF0001.RAF".to_owned(),
            10,
            "2026-10-17T14:05:09".to_owned(),
            "blake3:abab".to_owned(),
            "2026-10-17/DSCF0001.RAF".to_owned(),
            "2026-10-31T11:00:00+02:00".to_owned(),
        )
    );
}

/// spec: IMPORT-009.18
#[test]
fn a_run_has_its_source_destination_start_and_end() {
    let folder = TempDir::new();
    let history = SqliteImportHistory::new(folder.path());
    let run = history.start_run(&run_start()).unwrap();
    history.finish_run(run, now()).unwrap();
    drop(history);

    let connection = Connection::open(folder.path().join(HISTORY_FILE_NAME)).unwrap();
    let recorded: (i64, String, String, Vec<u8>, Vec<u8>) = connection
        .query_row(
            "SELECT id, started_at, finished_at, source, import_destination FROM runs",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        recorded,
        (
            run.number(),
            "2026-10-31T11:00:00+02:00".to_owned(),
            "2026-10-31T11:00:00+02:00".to_owned(),
            b"/media/card".to_vec(),
            b"/library".to_vec(),
        )
    );
}

/// spec: IMPORT-009.15
#[test]
fn a_database_of_another_version_is_an_error_and_unchanged() {
    let folder = TempDir::new();
    let file = folder.path().join(HISTORY_FILE_NAME);
    let connection = Connection::open(&file).unwrap();
    connection
        .execute_batch("CREATE TABLE copies (anything TEXT); PRAGMA user_version = 2;")
        .unwrap();
    drop(connection);
    let before = fs::read(&file).unwrap();
    let history = SqliteImportHistory::new(folder.path());

    let found = history.find_copies(&key(Path::new("/library")));
    let started = history.start_run(&run_start());

    let error = found.unwrap_err().to_string();
    assert!(error.contains("user_version is 2"), "{error}");
    assert!(started.is_err());
    drop(history);
    assert_eq!(fs::read(&file).unwrap(), before);
}

/// spec: IMPORT-009.15
#[test]
fn a_foreign_database_without_a_version_is_an_error_and_unchanged() {
    let folder = TempDir::new();
    let file = folder.path().join(HISTORY_FILE_NAME);
    let connection = Connection::open(&file).unwrap();
    connection
        .execute_batch("CREATE TABLE photos (name TEXT);")
        .unwrap();
    drop(connection);
    let before = fs::read(&file).unwrap();
    let history = SqliteImportHistory::new(folder.path());

    let started = history.start_run(&run_start());

    let error = started.unwrap_err().to_string();
    assert!(error.contains("holds tables"), "{error}");
    drop(history);
    assert_eq!(fs::read(&file).unwrap(), before);
}
