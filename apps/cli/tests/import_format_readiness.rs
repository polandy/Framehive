// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The format readiness at the end of `framehive import` (CLI-007), against the real
//! binary, the real file system and a temporary folder. The child process runs with
//! `TZ=UTC`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::process::Output;

use support::{DEFAULT_TARGET, Folders, add_folder_nested_too_deep, lines, stderr, stdout};

/// The last `count` lines of the standard output.
fn last_lines(output: &Output, count: usize) -> Vec<String> {
    let printed = lines(output);
    printed[printed.len().saturating_sub(count)..].to_vec()
}

fn backup_folder(folders: &Folders) -> String {
    folders.root.path().join("backup").display().to_string()
}

/// spec: CLI-007.1
#[test]
fn a_source_saved_at_every_destination_can_be_formatted() {
    let folders = Folders::new();
    folders.add_raf();
    let backup = backup_folder(&folders);

    let output = folders.import(&["--backup", &backup]);

    assert_eq!(
        last_lines(&output, 1),
        ["The source can be formatted: every file is verified in 2 locations."],
        "{}",
        stdout(&output)
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-007.2
#[test]
fn without_backup_destinations_one_location_is_required() {
    let folders = Folders::new();
    folders.add_raf();

    let output = folders.import(&[]);

    assert_eq!(
        last_lines(&output, 1),
        ["The source can be formatted: every file is verified in 1 location."],
        "{}",
        stdout(&output)
    );
}

/// spec: CLI-007.3
#[test]
fn a_failed_backup_is_listed() {
    let folders = Folders::new();
    folders.add_raf();
    let backup = backup_folder(&folders);
    let occupied = folders.root.path().join("backup").join(DEFAULT_TARGET);
    fs::create_dir_all(occupied.parent().unwrap()).unwrap();
    fs::write(&occupied, b"other content").unwrap();

    let output = folders.import(&["--backup", &backup]);

    assert_eq!(
        last_lines(&output, 2),
        [
            "Do not format the source: not every file is verified in 2 locations.",
            "  DSCF0001.RAF: verified in 1 of 2 locations",
        ],
        "{}",
        stdout(&output)
    );
    assert_eq!(output.status.code(), Some(1));
}

/// spec: CLI-007.4
#[test]
fn a_skipped_file_is_listed_and_the_exit_code_stays_0() {
    let folders = Folders::new();
    folders.add_raf();
    let existing = folders.destination().join(DEFAULT_TARGET);
    fs::create_dir_all(existing.parent().unwrap()).unwrap();
    fs::write(&existing, b"another photo").unwrap();

    let output = folders.import(&["--on-conflict", "skip"]);

    assert_eq!(
        last_lines(&output, 2),
        [
            "Do not format the source: not every file is verified in 1 location.",
            "  DSCF0001.RAF: verified in 0 of 1 location",
        ],
        "{}",
        stdout(&output)
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-007.5
#[test]
fn an_unreadable_folder_is_listed() {
    let folders = Folders::new();
    folders.add_raf();
    add_folder_nested_too_deep(&folders.source().join("locked"));

    let output = folders.import(&[]);

    let ending = last_lines(&output, 2);
    assert_eq!(
        ending[0],
        "Do not format the source: not every file is verified in 1 location.",
        "{}",
        stdout(&output)
    );
    assert!(
        ending[1].starts_with("  locked/") && ending[1].contains(": left out: cannot be read: "),
        "{}",
        stdout(&output)
    );
}

/// spec: CLI-007.6
#[test]
fn files_left_out_by_the_scan_are_counted() {
    let folders = Folders::new();
    folders.add_raf();
    folders.add("notes.txt", b"notes");

    let output = folders.import(&[]);

    assert_eq!(
        last_lines(&output, 2),
        [
            "The source can be formatted: every file is verified in 1 location.",
            "Not saved: 1 left out by the scan.",
        ],
        "{}",
        stdout(&output)
    );
}

/// spec: CLI-007.7
#[test]
fn a_dry_run_prints_no_format_readiness() {
    let folders = Folders::new();
    folders.add_raf();
    let backup = backup_folder(&folders);

    let output = folders.import(&["--backup", &backup, "--dry-run"]);

    assert!(
        lines(&output).iter().all(|line| {
            !line.starts_with("The source can be formatted") && !line.starts_with("Do not format")
        }),
        "{}",
        stdout(&output)
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}
