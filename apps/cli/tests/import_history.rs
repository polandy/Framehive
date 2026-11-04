// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` with the import history in the state folder (CLI-008, IMPORT-009)
//! against the real binary, the real file system and a temporary state folder.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::Path;

use support::{DEFAULT_TARGET, Folders, has_line, lines, stderr, stdout, summary_line};

const FIRST_RUN_SUMMARY: &str =
    "1 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out";
const SECOND_RUN_SUMMARY: &str =
    "0 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out";
const IMPORTED_BEFORE: &str = "imported before  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF";

/// The files below `folder`, relative to it, sorted.
fn files_below(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(current) = folders.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                folders.push(path);
            } else {
                let relative = path.strip_prefix(folder).unwrap();
                found.push(relative.display().to_string());
            }
        }
    }
    found.sort();
    found
}

/// spec: CLI-008.1
#[test]
fn a_second_run_with_a_persisted_counter_copies_nothing() {
    let folders = Folders::new();
    folders.add_raf();
    let template = ["--template", "{seq_stored:4}.{ext}"];
    let first = folders.import(&template);
    assert!(
        has_line(&first, "copied  DSCF0001.RAF -> 0001.RAF"),
        "{}",
        stdout(&first)
    );

    let output = folders.import(&template);

    assert!(
        has_line(&output, "imported before  DSCF0001.RAF -> 0001.RAF"),
        "{}",
        stdout(&output)
    );
    assert_eq!(summary_line(&output), SECOND_RUN_SUMMARY);
    let counters = fs::read_to_string(folders.counter_file()).unwrap();
    assert!(counters.contains("stored_last=1\n"), "{counters}");
    assert_eq!(stderr(&output), "");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(files_below(&folders.destination()), ["0001.RAF"]);
}

/// spec: CLI-008.2
#[test]
fn a_file_renamed_in_the_library_is_imported_before() {
    let folders = Folders::new();
    folders.add_raf();
    folders.import(&[]);
    let library = folders.destination();
    fs::rename(
        library.join(DEFAULT_TARGET),
        library.join("2026/2026-10-17/best.RAF"),
    )
    .unwrap();

    let output = folders.import(&[]);

    assert!(has_line(&output, IMPORTED_BEFORE), "{}", stdout(&output));
    assert_eq!(summary_line(&output), SECOND_RUN_SUMMARY);
    assert_eq!(files_below(&library), ["2026/2026-10-17/best.RAF"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-008.3
#[test]
fn a_file_backed_up_before_is_reported_below_its_line() {
    let folders = Folders::new();
    folders.add_raf();
    let backup = folders.root.path().join("backup").display().to_string();
    folders.import(&["--backup", &backup]);

    let output = folders.import(&["--backup", &backup]);

    let printed = lines(&output);
    let imported = printed
        .iter()
        .position(|line| line == IMPORTED_BEFORE)
        .unwrap_or_else(|| panic!("{}", stdout(&output)));
    assert_eq!(
        printed[imported + 1],
        format!("  backed up before  {backup}")
    );
    assert!(
        has_line(
            &output,
            &format!("backup {backup}: 0 backed up, 1 already backed up, 0 failed")
        ),
        "{}",
        stdout(&output)
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-008.4
#[test]
fn the_history_is_kept_in_the_state_folder() {
    let folders = Folders::new();
    folders.add_raf();
    fs::create_dir(folders.root.path().join("state")).unwrap();

    let output = folders.import(&[]);

    assert_eq!(summary_line(&output), FIRST_RUN_SUMMARY);
    assert!(folders.history_file().is_file());
}

/// spec: IMPORT-009.14
#[test]
fn a_dry_run_creates_no_history_file() {
    let folders = Folders::new();
    folders.add_raf();

    let output = folders.import(&["--dry-run"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(!folders.history_file().exists());
    assert!(!folders.history_file().parent().unwrap().exists());
}

/// spec: CLI-008.5
#[test]
fn a_history_that_is_not_a_database_stops_the_import() {
    let folders = Folders::new();
    folders.add_raf();
    let history = folders.history_file();
    fs::create_dir_all(history.parent().unwrap()).unwrap();
    let garbage = b"this is not a SQLite database, it is only text".repeat(100);
    fs::write(&history, &garbage).unwrap();

    let output = folders.import(&[]);

    let expected = format!(
        "error: the import history {} cannot be used: ",
        history.display()
    );
    assert!(
        stderr(&output).starts_with(&expected),
        "{}",
        stderr(&output)
    );
    assert!(files_below(&folders.destination()).is_empty());
    assert_eq!(fs::read(&history).unwrap(), garbage);
    assert_eq!(output.status.code(), Some(2));
}

/// The destination is identified by its canonical path (IMPORT-009): a later run
/// through a symbolic link to the library recognizes the files of the first run.
#[test]
fn a_destination_named_through_a_link_is_the_same_destination() {
    let folders = Folders::new();
    folders.add_raf();
    folders.import(&[]);
    let link = folders.root.path().join("link-to-library");
    std::os::unix::fs::symlink(folders.destination(), &link).unwrap();
    fs::remove_file(folders.destination().join(DEFAULT_TARGET)).unwrap();

    let output = folders.import_into(&link, &[]);

    assert!(has_line(&output, IMPORTED_BEFORE), "{}", stdout(&output));
    assert_eq!(summary_line(&output), SECOND_RUN_SUMMARY);
}
