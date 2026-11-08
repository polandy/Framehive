// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! How `framehive import-log` (CLI-009) prints names that are not plain text: paths that
//! are not valid UTF-8, and names with control characters, on a history written with
//! `adapter-sqlite` in a temporary state folder.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::path::PathBuf;

use adapter_sqlite::SqliteImportHistory;
use domain::{Checksum, Timestamp, UtcOffset};
use history::{CopyRecord, ImportHistory, RecordedTime, RunStart};
use support::{Folders, stdout};

fn recorded_time() -> RecordedTime {
    RecordedTime::at(
        Timestamp::from_unix_seconds(1_793_437_200),
        UtcOffset::from_seconds(0).unwrap(),
    )
    .unwrap()
}

/// Records run 1 from `source` into `/library` with one copy of `source_path` to
/// `target`, which is also its original file name, as an import writes it.
fn record_run(folders: &Folders, source: PathBuf, source_path: PathBuf, target: &str) {
    let history = SqliteImportHistory::new(folders.history_file().parent().unwrap());
    let run = history
        .start_run(&RunStart {
            source,
            import_destination: PathBuf::from("/library"),
            video_destination: None,
            started_at: recorded_time(),
        })
        .unwrap();
    history
        .record_copy(&CopyRecord {
            run,
            destination: PathBuf::from("/library"),
            source_path,
            original_file_name: target.to_owned(),
            size: 1,
            capture_time: None,
            checksum: Checksum::new(Checksum::BLAKE3, vec![1]).unwrap(),
            target: target.to_owned(),
            recorded_at: recorded_time(),
            sidecar: None,
        })
        .unwrap();
}

/// Paths that are not valid UTF-8 are printed with U+FFFD in every format (CLI-009,
/// "Paths").
#[test]
fn a_path_that_is_not_utf8_is_printed_with_replacement_characters() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let folders = Folders::new();
    record_run(
        &folders,
        PathBuf::from(OsStr::from_bytes(b"/media/\xff")),
        PathBuf::from(OsStr::from_bytes(b"\xfe.jpg")),
        "x.jpg",
    );

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

/// spec: CLI-009.15
#[test]
fn a_name_with_a_line_break_stays_on_one_line_in_text() {
    let folders = Folders::new();
    record_run(
        &folders,
        PathBuf::from("/media/a\nb"),
        PathBuf::from("x\ny.jpg"),
        "x\ry.jpg",
    );

    let list = stdout(&folders.import_log(&[]));
    let text = stdout(&folders.import_log(&["1"]));

    assert!(
        list.lines()
            .any(|line| line.ends_with("/media/a\\nb -> /library")),
        "{list}"
    );
    assert!(
        text.lines()
            .any(|line| line == "  x\\ny.jpg -> x\\ry.jpg  blake3:01"),
        "{text}"
    );
}
