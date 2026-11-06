// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import-log` (CLI-009) against the real binary, after real imports or on a
//! history written with `adapter-sqlite`, in a temporary state folder.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use adapter_sqlite::SqliteImportHistory;
use domain::{Checksum, Timestamp, UtcOffset};
use history::{CopyRecord, ImportHistory, RecordedTime, RunStart};
use support::{DEFAULT_TARGET, Folders, lines, photo, stderr, stdout};

/// The line of the copy of `DSCF0001.RAF` in the log of a run, without its checksum.
const COPY_LINE: &str = "  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF  blake3:";

fn canonical(folder: &Path) -> String {
    fs::canonicalize(folder).unwrap().display().to_string()
}

/// Whether `text` is a time as the history records it in the time zone UTC.
fn is_recorded_time(text: &str) -> bool {
    text.len() == 25 && text.as_bytes()[10] == b'T' && text.ends_with("+00:00")
}

/// The fields of a line of the list of runs, split at the two spaces between them.
fn fields(line: &str) -> Vec<&str> {
    line.split("  ").collect()
}

/// The history of the state folder, written as an import writes it.
fn history(folders: &Folders) -> SqliteImportHistory {
    SqliteImportHistory::new(folders.history_file().parent().unwrap())
}

fn recorded_time() -> RecordedTime {
    RecordedTime::at(
        Timestamp::from_unix_seconds(1_793_437_200),
        UtcOffset::from_seconds(0).unwrap(),
    )
    .unwrap()
}

/// spec: CLI-009.1
#[test]
fn the_runs_are_listed_oldest_first() {
    let folders = Folders::new();
    folders.add_raf();
    folders.import(&[]);
    let backup = folders.root.path().join("backup");
    folders.import(&["--backup", &backup.display().to_string()]);

    let output = folders.import_log(&[]);

    let printed = lines(&output);
    assert_eq!(printed.len(), 2, "{}", stdout(&output));
    let folders_text = format!(
        "{} -> {}",
        canonical(&folders.source()),
        canonical(&folders.destination())
    );
    for (line, (number, copies)) in printed.iter().zip([("1", "1 copies"), ("2", "1 copies")]) {
        let parts = fields(line);
        assert_eq!(parts.len(), 5, "{line}");
        assert_eq!(parts[0], number, "{line}");
        assert!(is_recorded_time(parts[1]), "{line}");
        assert!(is_recorded_time(parts[2]), "{line}");
        assert_eq!(parts[3], copies, "{line}");
        assert_eq!(parts[4], folders_text, "{line}");
    }
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-009.2
#[test]
fn a_run_without_an_end_is_not_finished() {
    let folders = Folders::new();
    history(&folders)
        .start_run(&RunStart {
            source: PathBuf::from("/media/card"),
            import_destination: PathBuf::from("/library"),
            video_destination: None,
            started_at: recorded_time(),
        })
        .unwrap();

    let output = folders.import_log(&[]);

    assert_eq!(
        stdout(&output),
        "1  2026-10-31T09:00:00+00:00  not finished  0 copies  /media/card -> /library\n"
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-009.3
#[test]
fn an_empty_state_folder_has_no_runs_and_stays_empty() {
    let folders = Folders::new();
    let state = folders.root.path().join("state");
    fs::create_dir(&state).unwrap();

    let output = folders.import_log(&[]);

    assert_eq!(stdout(&output), "No import runs recorded.\n");
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(fs::read_dir(&state).unwrap().count(), 0);
}

/// spec: CLI-009.4
#[test]
fn the_log_of_one_run_has_a_block_per_destination() {
    let folders = Folders::new();
    folders.add_raf();
    let backup = folders.root.path().join("backup");
    folders.import(&["--backup", &backup.display().to_string()]);

    let output = folders.import_log(&["1"]);

    let printed = lines(&output);
    assert_eq!(printed[0], "run 1", "{}", stdout(&output));
    let source = format!("source               {}", canonical(&folders.source()));
    let destination = canonical(&folders.destination());
    assert_eq!(printed[3], source);
    assert_eq!(printed[4], format!("import destination   {destination}"));
    assert_eq!(printed[5], "");
    assert_eq!(printed[6], destination);
    assert!(printed[7].starts_with(COPY_LINE), "{}", printed[7]);
    assert_eq!(printed[8], "");
    assert_eq!(printed[9], canonical(&backup));
    assert_eq!(printed[10], printed[7]);
    assert_eq!(printed.len(), 11, "{}", stdout(&output));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-009.5
#[test]
fn last_is_the_run_with_the_highest_number() {
    let folders = Folders::new();
    folders.add_raf();
    folders.import(&[]);
    folders.import(&[]);

    let output = folders.import_log(&["last"]);

    assert_eq!(lines(&output)[0], "run 2", "{}", stdout(&output));
    assert!(stdout(&output).ends_with("\nNo copies recorded.\n"));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-009.6
#[test]
fn the_log_of_one_run_as_json() {
    let folders = Folders::new();
    folders.add_raf();
    folders.import(&[]);

    let output = folders.import_log(&["1", "--format", "json"]);

    let json = stdout(&output);
    assert!(
        json.starts_with("{\"run\": 1, \"started_at\": \""),
        "{json}"
    );
    assert!(json.ends_with("}]}\n"), "{json}");
    assert_eq!(json.lines().count(), 1, "{json}");
    for field in [
        "\"finished_at\": \"",
        "\"source\": ",
        "\"import_destination\": ",
        "\"original_file_name\": \"DSCF0001.RAF\"",
        "\"size\": ",
        "\"capture_time\": \"2026-10-17T14:05:09\"",
        "\"checksum\": \"blake3:",
        &format!("\"target\": \"{DEFAULT_TARGET}\""),
        "\"recorded_at\": \"",
    ] {
        assert!(json.contains(field), "{field} missing in {json}");
    }
    assert_eq!(json.matches("\"destination\": ").count(), 1, "{json}");
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-009.7
#[test]
fn a_file_without_a_capture_time_has_a_null_capture_time() {
    let folders = Folders::new();
    folders.add("notes.png", b"\x89PNG\r\n\x1a\nnot a real image");
    folders.import(&[]);

    let output = folders.import_log(&["1", "--format", "json"]);

    let json = stdout(&output);
    assert!(
        json.contains("\"original_file_name\": \"notes.png\""),
        "{json}"
    );
    assert!(json.contains("\"capture_time\": null"), "{json}");
}

/// spec: CLI-009.8
#[test]
fn the_log_of_one_run_as_csv() {
    let folders = Folders::new();
    folders.add_raf();
    let backup = folders.root.path().join("backup");
    folders.import(&["--backup", &backup.display().to_string()]);

    let output = folders.import_log(&["1", "--format", "csv"]);

    let csv = stdout(&output);
    let rows: Vec<&str> = csv.split_inclusive("\r\n").collect();
    assert_eq!(rows.len(), 3, "{csv}");
    assert_eq!(
        rows[0],
        "run,destination,source_path,original_file_name,size,capture_time,checksum,target,recorded_at\r\n"
    );
    let library = format!(
        "1,{},DSCF0001.RAF,DSCF0001.RAF,",
        canonical(&folders.destination())
    );
    assert!(rows[1].starts_with(&library), "{csv}");
    assert!(
        rows[2].starts_with(&format!("1,{},", canonical(&backup))),
        "{csv}"
    );
    assert!(rows.iter().all(|row| row.ends_with("\r\n")));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-009.9
#[test]
fn a_csv_field_with_a_comma_and_quotes_is_quoted() {
    let folders = Folders::new();
    folders.add("a,\"b\".jpg", &photo());
    folders.import(&[]);

    let output = folders.import_log(&["1", "--format", "csv"]);

    let csv = stdout(&output);
    assert!(
        csv.contains(",\"a,\"\"b\"\".jpg\",\"a,\"\"b\"\".jpg\","),
        "{csv}"
    );
}

/// spec: CLI-009.10
#[test]
fn a_run_that_does_not_exist_is_an_error() {
    let folders = Folders::new();
    folders.add_raf();
    folders.import(&[]);

    let output = folders.import_log(&["7"]);

    assert_eq!(stderr(&output), "error: no import run 7\n");
    assert_eq!(stdout(&output), "");
    assert_eq!(output.status.code(), Some(2));
}

/// `last` without any run is an error (CLI-009, "Errors").
#[test]
fn last_without_runs_is_an_error() {
    let folders = Folders::new();

    let output = folders.import_log(&["last"]);

    assert_eq!(stderr(&output), "error: no import runs recorded\n");
    assert_eq!(output.status.code(), Some(2));
    assert!(!folders.root.path().join("state").exists());
}

/// spec: CLI-009.11
#[test]
fn a_history_that_is_not_a_database_is_an_error_and_unchanged() {
    let folders = Folders::new();
    let file = folders.history_file();
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    let garbage = b"this is not a SQLite database, it is only text".repeat(100);
    fs::write(&file, &garbage).unwrap();

    let output = folders.import_log(&[]);

    let expected = format!(
        "error: the import history {} cannot be used: ",
        file.display()
    );
    assert!(
        stderr(&output).starts_with(&expected),
        "{}",
        stderr(&output)
    );
    assert_eq!(fs::read(&file).unwrap(), garbage);
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-009.12
#[test]
fn format_without_a_run_is_a_usage_error() {
    let folders = Folders::new();

    for format in ["csv", "text"] {
        let output = folders.import_log(&["--format", format]);

        assert_eq!(output.status.code(), Some(2), "{format}");
        assert_eq!(stdout(&output), "", "{format}");
        assert!(stderr(&output).contains("<run>"), "{}", stderr(&output));
    }
}

/// Without a state folder the command stops as the import does (CLI-008, CLI-009).
#[test]
fn without_a_state_folder_the_command_stops() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_framehive"))
        .arg("import-log")
        .env_remove("XDG_STATE_HOME")
        .env("HOME", "not-absolute")
        .output()
        .unwrap();

    assert_eq!(
        stderr(&output),
        "error: the home folder is unknown; set HOME\n"
    );
    assert_eq!(output.status.code(), Some(2));
}

/// Paths that are not valid UTF-8 are printed with U+FFFD in every format (CLI-009,
/// "Paths").
#[test]
fn a_path_that_is_not_utf8_is_printed_with_replacement_characters() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let folders = Folders::new();
    let history = history(&folders);
    let run = history
        .start_run(&RunStart {
            source: PathBuf::from(OsStr::from_bytes(b"/media/\xff")),
            import_destination: PathBuf::from("/library"),
            video_destination: None,
            started_at: recorded_time(),
        })
        .unwrap();
    history
        .record_copy(&CopyRecord {
            run,
            destination: PathBuf::from("/library"),
            source_path: PathBuf::from(OsStr::from_bytes(b"\xfe.jpg")),
            original_file_name: "x.jpg".to_owned(),
            size: 1,
            capture_time: None,
            checksum: Checksum::new(Checksum::BLAKE3, vec![1]).unwrap(),
            target: "x.jpg".to_owned(),
            recorded_at: recorded_time(),
        })
        .unwrap();
    drop(history);

    let list = stdout(&folders.import_log(&[]));
    let text = stdout(&folders.import_log(&["1"]));
    let json = stdout(&folders.import_log(&["1", "--format", "json"]));
    let csv = stdout(&folders.import_log(&["1", "--format", "csv"]));

    assert!(list.contains("/media/\u{fffd} -> /library"), "{list}");
    assert!(
        text.contains("  \u{fffd}.jpg -> x.jpg  blake3:01"),
        "{text}"
    );
    assert!(json.contains("\"source\": \"/media/\u{fffd}\""), "{json}");
    assert!(csv.contains(",\u{fffd}.jpg,x.jpg,"), "{csv}");
}
