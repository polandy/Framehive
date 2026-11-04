// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` (CLI-004) with a persisted sequence counter (IMPORT-002,
//! ADR 0029): reserved numbers, dry runs and numbers that cannot be reserved.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use support::{Folders, has_line_starting_with, stderr};

const COUNTERS_AT_250: &str = "format_version=1\nstored_last=250\n";
const TEMPLATE: &str = "{seq_stored:4}.{ext}";
const NOT_RECOGNIZED: &str = "warning: files already imported are not recognized, because the name template has a persisted sequence counter";

fn folders_with_two_videos(counters: &str) -> Folders {
    let folders = Folders::new();
    folders.add("a.MOV", b"first video");
    folders.add("b.MOV", b"second video");
    folders.write_counters(counters);
    folders
}

fn counters(folders: &Folders) -> String {
    fs::read_to_string(folders.counter_file()).unwrap()
}

/// How many files the import wrote to the destination; it may have created the folder.
fn copied_files(folders: &Folders) -> usize {
    fs::read_dir(folders.destination()).map_or(0, Iterator::count)
}

/// spec: CLI-004.1
#[test]
fn copies_with_reserved_numbers_and_warns() {
    let folders = folders_with_two_videos(COUNTERS_AT_250);

    let output = folders.import(&["--template", TEMPLATE]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let library = folders.destination();
    assert_eq!(fs::read(library.join("0251.MOV")).unwrap(), b"first video");
    assert_eq!(fs::read(library.join("0252.MOV")).unwrap(), b"second video");
    assert!(counters(&folders).contains("stored_last=252\n"));
    assert!(
        stderr(&output).lines().any(|line| line == NOT_RECOGNIZED),
        "{}",
        stderr(&output)
    );
}

/// spec: CLI-004.2
#[test]
fn a_dry_run_shows_the_numbers_and_reserves_none() {
    let folders = folders_with_two_videos(COUNTERS_AT_250);

    let output = folders.import(&["--template", TEMPLATE, "--dry-run"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(has_line_starting_with(
        &output,
        "would copy  a.MOV -> 0251.MOV"
    ));
    assert!(has_line_starting_with(
        &output,
        "would copy  b.MOV -> 0252.MOV"
    ));
    assert_eq!(counters(&folders), COUNTERS_AT_250);
    assert!(!folders.destination().exists());
}

/// spec: CLI-004.3
#[test]
fn numbers_that_do_not_fit_stop_the_import() {
    let text = "format_version=1\nstored_last=99\n";
    let folders = folders_with_two_videos(text);

    let output = folders.import(&["--template", "{seq_stored:2}.{ext}"]);

    assert_eq!(output.status.code(), Some(2));
    let error = stderr(&output);
    assert!(error.contains("{seq_stored:3}.{ext}"), "{error}");
    assert_eq!(counters(&folders), text);
    assert_eq!(copied_files(&folders), 0);
}

/// spec: CLI-004.3
#[test]
fn broken_counters_stop_the_import() {
    let text = "format_version=1\nstored_last=many\n";
    let folders = folders_with_two_videos(text);

    let output = folders.import(&["--template", "{seq_stored:2}.{ext}"]);

    assert_eq!(output.status.code(), Some(2));
    let error = stderr(&output);
    assert!(
        error.contains(&folders.counter_file().display().to_string()),
        "{error}"
    );
    assert!(error.contains("line 2"), "{error}");
    assert_eq!(counters(&folders), text);
    assert_eq!(copied_files(&folders), 0);
}
