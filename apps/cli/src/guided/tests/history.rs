// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import history in the guided import (CLI-008, IMPORT-009).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use adapter_sqlite::SqliteImportHistory;
use files::testing::TempDir;

use super::support::{Harness, OCTOBER_17, PICTURES, Step};
use crate::paths::PathError;

const MEGABYTES_10: usize = 10_000_000;

/// A harness with `template`, the clock advancing one second per copy, and the card
/// `EOS_DIGITAL` with three files of 10 MB.
fn with_three_files(template: &str) -> Harness {
    let harness = Harness::new();
    harness.remember(Some(PICTURES), Some(template), &[]);
    harness.insert("EOS_DIGITAL");
    for number in 1..=3u8 {
        let path = format!("DCIM/100_FUJI/DSCF000{number}.RAF");
        harness.add_file(
            "EOS_DIGITAL",
            &path,
            &vec![number; MEGABYTES_10],
            OCTOBER_17,
        );
    }
    harness.time_per_copy.set(Duration::from_secs(1));
    harness
}

/// The files in the library, sorted.
fn library_files(harness: &Harness) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = harness
        .memory
        .paths()
        .into_iter()
        .filter(|path| path.starts_with(PICTURES) && harness.memory.content(path).is_some())
        .collect();
    files.sort();
    files
}

/// spec: CLI-008.6
#[test]
fn a_pulled_card_is_resumed_from_the_history() {
    let harness = with_three_files("{seq_stored:4}.{ext}");
    harness.pull_after_copies.set(Some(1));

    let transcript = harness.run(vec![
        Step::Line(""),
        Step::Tick,
        Step::Insert("EOS_DIGITAL"),
        Step::Tick,
        Step::Line("n"),
    ]);

    let output = &transcript.output;
    assert!(
        output.contains(
            "\n2 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n"
        ),
        "{output}"
    );
    assert!(!output.contains("not recognized"), "{output}");
    let library = Path::new(PICTURES);
    // The first run reserved 1 to 3 and copied the first file as 0001; the second run
    // reserves 4 and 5 for the two files it copies.
    assert_eq!(
        library_files(&harness),
        [
            library.join("0001.RAF"),
            library.join("0004.RAF"),
            library.join("0005.RAF")
        ]
    );
    for (name, number) in [("0001.RAF", 1), ("0004.RAF", 2), ("0005.RAF", 3)] {
        assert_eq!(
            harness.memory.content(&library.join(name)),
            Some(vec![number; MEGABYTES_10])
        );
    }
}

/// spec: CLI-008.7
#[test]
fn a_history_that_cannot_be_used_keeps_the_card_from_being_imported() {
    let state = TempDir::new().unwrap();
    let history = SqliteImportHistory::new(state.path());
    std::fs::write(history.file(), b"not a SQLite database".repeat(100)).unwrap();
    let file = history.file().display().to_string();
    let mut harness = with_three_files("{name}.{ext}");
    harness.history = Arc::new(history);

    let transcript = harness.run(vec![Step::Line("")]);

    let expected = format!("\nThe import history {file} cannot be used: ");
    assert!(
        transcript.output.contains(&expected),
        "{}",
        transcript.output
    );
    assert!(
        library_files(&harness).is_empty(),
        "{:?}",
        library_files(&harness)
    );
}

/// The template preview shows a file imported before with the line of the file list
/// (CLI-008).
#[test]
fn the_preview_shows_a_file_imported_before() {
    let harness = with_three_files("{name}.{ext}");
    harness.run(vec![Step::Line(""), Step::Line("n")]);
    harness.remember(Some(PICTURES), None, &[]);

    let transcript = harness.run(vec![Step::Line("1")]);

    assert!(
        transcript
            .output
            .contains("\x20 imported before  DCIM/100_FUJI/DSCF0001.RAF -> DSCF0001.RAF\n"),
        "{}",
        transcript.output
    );
}

/// A folder resolution that always fails, as for a destination that cannot be read.
fn unresolvable(folder: &Path) -> Result<PathBuf, PathError> {
    Err(PathError::DestinationCannotBeRead(
        folder.to_path_buf(),
        io::Error::from(io::ErrorKind::PermissionDenied),
    ))
}

/// spec: CLI-008.8
#[test]
fn a_folder_that_cannot_be_resolved_keeps_the_card_from_being_imported() {
    let mut harness = with_three_files("{name}.{ext}");
    harness.resolve_folder = unresolvable;

    let transcript = harness.run(vec![Step::Line("")]);

    let expected = format!("\nCould not use {PICTURES}: ");
    assert!(
        transcript.output.contains(&expected),
        "{}",
        transcript.output
    );
    assert!(
        library_files(&harness).is_empty(),
        "{:?}",
        library_files(&harness)
    );
}

/// spec: CLI-008.9
#[test]
fn a_folder_that_cannot_be_resolved_skips_the_template_preview() {
    let mut harness = with_three_files("{name}.{ext}");
    harness.remember(Some(PICTURES), None, &[]);
    harness.resolve_folder = unresolvable;

    let transcript = harness.run(vec![Step::Line("1"), Step::Line("n")]);

    assert!(
        transcript
            .output
            .contains("The template cannot be previewed: "),
        "{}",
        transcript.output
    );
    assert!(
        transcript.output.contains("Use this template? [Y/n]: "),
        "{}",
        transcript.output
    );
}
