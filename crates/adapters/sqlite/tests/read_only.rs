// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import history opened read-only for the import log (IMPORT-010, ADR 0031,
//! section 7): it never creates or changes the database or the state folder.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use adapter_sqlite::{HISTORY_FILE_NAME, SqliteImportHistory};
use domain::{Timestamp, UtcOffset};
use history::{ImportHistory, RecordedTime, RunId, RunStart};
use rusqlite::Connection;
use support::TempDir;

fn now() -> RecordedTime {
    RecordedTime::at(
        Timestamp::from_unix_seconds(1_793_437_200),
        UtcOffset::from_seconds(7_200).unwrap(),
    )
    .unwrap()
}

fn run_start() -> RunStart {
    RunStart {
        source: PathBuf::from("/media/card"),
        import_destination: PathBuf::from("/library"),
        video_destination: None,
        started_at: now(),
    }
}

/// The names of the entries of `folder`, sorted.
fn entries(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// spec: IMPORT-010.7
#[test]
fn listing_a_missing_history_creates_nothing() {
    let folder = TempDir::new();
    let state_folder = folder.path().join("framehive");
    let history = SqliteImportHistory::read_only(&state_folder);

    let runs = history.list_runs().unwrap();
    let log = history.read_run_log(RunId::new(1)).unwrap();

    assert!(runs.is_empty());
    assert!(log.is_none());
    assert!(!state_folder.exists());
}

/// spec: IMPORT-010.8
#[test]
fn a_history_that_is_not_a_database_is_an_error_that_names_it() {
    let folder = TempDir::new();
    let file = folder.path().join(HISTORY_FILE_NAME);
    let garbage = b"this is not a SQLite database, it is only text".repeat(100);
    fs::write(&file, &garbage).unwrap();
    let history = SqliteImportHistory::read_only(folder.path());

    let error = history.list_runs().unwrap_err();

    assert_eq!(error.file, file);
    assert!(
        error.to_string().contains(&file.display().to_string()),
        "{error}"
    );
    drop(history);
    assert_eq!(fs::read(&file).unwrap(), garbage);
    assert_eq!(entries(folder.path()), [HISTORY_FILE_NAME]);
}

/// spec: IMPORT-010.8
#[test]
fn a_history_of_another_version_is_an_error_and_unchanged() {
    let folder = TempDir::new();
    let file = folder.path().join(HISTORY_FILE_NAME);
    let connection = Connection::open(&file).unwrap();
    connection
        .execute_batch("CREATE TABLE runs (anything TEXT); PRAGMA user_version = 3;")
        .unwrap();
    drop(connection);
    let before = fs::read(&file).unwrap();
    let history = SqliteImportHistory::read_only(folder.path());

    let error = history.read_run_log(RunId::new(1)).unwrap_err().to_string();

    assert!(error.contains("user_version is 3"), "{error}");
    drop(history);
    assert_eq!(fs::read(&file).unwrap(), before);
}

/// spec: IMPORT-010.10
#[test]
fn a_read_only_history_reads_what_was_recorded_and_leaves_no_side_files() {
    let folder = TempDir::new();
    let writer = SqliteImportHistory::new(folder.path());
    let run = writer.start_run(&run_start()).unwrap();
    writer.finish_run(run, now()).unwrap();
    drop(writer);
    let before = entries(folder.path());
    let file = folder.path().join(HISTORY_FILE_NAME);
    let content = fs::read(&file).unwrap();
    let history = SqliteImportHistory::read_only(folder.path());

    let runs = history.list_runs().unwrap();
    drop(history);

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, run);
    assert_eq!(runs[0].finished_at, Some(now()));
    assert_eq!(entries(folder.path()), before);
    assert_eq!(fs::read(&file).unwrap(), content);
}

/// A history opened read-only refuses to record, and creates nothing.
#[test]
fn a_read_only_history_refuses_to_write() {
    let folder = TempDir::new();
    let state_folder = folder.path().join("framehive");
    let history = SqliteImportHistory::read_only(&state_folder);

    let started = history.start_run(&run_start());

    let error = started.unwrap_err();
    assert_eq!(error.file, state_folder.join(HISTORY_FILE_NAME));
    assert!(!state_folder.exists());
}
