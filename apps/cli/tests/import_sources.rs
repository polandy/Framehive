// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` with several sources (CLI-015). The sources are folders below
//! the test's root, named relative to it as the user would give them.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::PathBuf;
use std::process::Output;

use support::{Folders, has_line, has_line_starting_with, lines, photo, stderr, stdout};

const SUMMARY: &str = "1 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out";
const DAY: &str = "2026/2026-10-17";

/// Adds a source folder `card` below the root, with a picture captured at
/// 2026-10-17 14:05:09 for each of `names`; each picture's content is its own.
fn card(folders: &Folders, card: &str, names: &[&str]) {
    let folder = folders.root.path().join(card);
    fs::create_dir_all(&folder).unwrap();
    for name in names {
        fs::write(folder.join(name), picture(name)).unwrap();
    }
}

/// A picture that differs from every other by `name`, after the end of its image.
fn picture(name: &str) -> Vec<u8> {
    let mut bytes = photo();
    bytes.extend(name.as_bytes());
    bytes
}

/// Runs `framehive import <arguments>` in the root folder.
fn import(folders: &Folders, arguments: &[&str]) -> Output {
    let mut line = vec!["import"];
    line.extend(arguments);
    folders
        .framehive_command(&line)
        .current_dir(folders.root.path())
        .output()
        .unwrap()
}

fn library(folders: &Folders) -> PathBuf {
    folders.root.path().join("library")
}

/// The line after `line` in the output.
fn line_after(output: &Output, line: &str) -> Option<String> {
    let lines = lines(output);
    let index = lines.iter().position(|each| each == line)?;
    lines.get(index + 1).cloned()
}

/// The content of the file in `folder` named `<stem>.<extension>`.
fn content_of(folder: &PathBuf, stem: &str) -> Vec<u8> {
    let entry = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.file_stem().is_some_and(|found| found == stem))
        .unwrap();
    fs::read(entry).unwrap()
}

/// spec: CLI-015.1
#[test]
fn two_sources_are_imported_each_with_its_summary() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);
    card(&folders, "cardB", &["b.jpg"]);

    let output = import(&folders, &["cardA", "cardB", "library"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        fs::read(library(&folders).join(DAY).join("a.jpg")).unwrap(),
        picture("a.jpg")
    );
    assert_eq!(
        fs::read(library(&folders).join(DAY).join("b.jpg")).unwrap(),
        picture("b.jpg")
    );
    for source in ["cardA:", "cardB:"] {
        assert_eq!(
            line_after(&output, source).as_deref(),
            Some(SUMMARY),
            "{}",
            stdout(&output)
        );
    }
}

/// spec: CLI-015.2
#[test]
fn the_exit_code_is_the_worst_of_the_sources() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);
    card(&folders, "cardB", &["b.jpg"]);
    let taken = library(&folders).join(DAY).join("b.jpg");
    fs::create_dir_all(taken.parent().unwrap()).unwrap();
    fs::write(&taken, b"another file").unwrap();

    let output = import(
        &folders,
        &["cardA", "cardB", "library", "--on-conflict", "fail"],
    );

    assert_eq!(
        fs::read(library(&folders).join(DAY).join("a.jpg")).unwrap(),
        picture("a.jpg")
    );
    assert_eq!(fs::read(&taken).unwrap(), b"another file");
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
}

/// spec: CLI-015.3
#[test]
fn a_source_that_cannot_start_keeps_no_other_from_importing() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);

    let output = import(&folders, &["cardA", "missing", "library"]);

    assert_eq!(
        fs::read(library(&folders).join(DAY).join("a.jpg")).unwrap(),
        picture("a.jpg")
    );
    assert!(
        stderr(&output)
            .lines()
            .any(|line| line.starts_with("error: missing: ")),
        "{}",
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-015.4
#[test]
fn one_source_prints_no_source_line() {
    let folders = Folders::new();
    folders.add_raf();

    let output = folders.import(&[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(
        has_line_starting_with(&output, "1 copied"),
        "{}",
        stdout(&output)
    );
    let source = format!("{}:", folders.source().display());
    assert!(!has_line(&output, &source), "{}", stdout(&output));
    assert!(
        lines(&output).iter().all(|line| !line.ends_with(':')),
        "{}",
        stdout(&output)
    );
}

/// spec: CLI-015.5
#[test]
fn sources_continue_each_others_sequence_numbers() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg", "b.jpg"]);
    card(&folders, "cardB", &["c.jpg"]);
    folders.write_counters("format_version=1\nstored_last=0\n");

    let output = import(
        &folders,
        &[
            "cardA",
            "cardB",
            "library",
            "--template",
            "{seq_stored:3}.{ext}",
        ],
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let library = library(&folders);
    let mut first_card = [content_of(&library, "001"), content_of(&library, "002")];
    first_card.sort();
    let mut expected = [picture("a.jpg"), picture("b.jpg")];
    expected.sort();
    assert_eq!(first_card, expected);
    assert_eq!(content_of(&library, "003"), picture("c.jpg"));
}

/// spec: CLI-015.6
#[test]
fn an_argument_error_starts_no_source() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);
    card(&folders, "cardB", &["b.jpg"]);

    let output = import(
        &folders,
        &[
            "cardA",
            "cardB",
            "library",
            "--template",
            "{job}/{name}.{ext}",
        ],
    );

    assert!(stderr(&output).contains("{job}"), "{}", stderr(&output));
    assert!(!library(&folders).exists());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-015.7
#[test]
fn with_a_profile_the_last_of_several_folders_is_the_destination() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);
    card(&folders, "cardB", &["b.jpg"]);
    let holiday_library = folders.root.path().join("holiday-library");
    let profile = format!(
        "schema_version: 1\ndestination: \"{}\"\n",
        holiday_library.display()
    );
    folders.write_profile("holiday", profile.as_bytes());

    let output = import(
        &folders,
        &["cardA", "cardB", "library", "--profile", "holiday"],
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        fs::read(library(&folders).join(DAY).join("a.jpg")).unwrap(),
        picture("a.jpg")
    );
    assert_eq!(
        fs::read(library(&folders).join(DAY).join("b.jpg")).unwrap(),
        picture("b.jpg")
    );
    assert!(!holiday_library.exists());
}
