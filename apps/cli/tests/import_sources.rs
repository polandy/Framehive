// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` with several sources (CLI-015). The sources are folders below
//! the test's root, named relative to it as the user would give them.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use support::{
    Folders, card, has_line, has_line_starting_with, import_in_root as import, lines, picture,
    stderr, stdout,
};

const SUMMARY: &str = "1 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out";
const DAY: &str = "2026/2026-10-17";

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

    let output = import(&folders, &["cardA", "cardB", "--destination", "library"]);

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
        &[
            "cardA",
            "cardB",
            "--destination",
            "library",
            "--on-conflict",
            "fail",
        ],
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
    // A camera on a port of a missing folder cannot be opened by any user, also root.
    let camera = format!(
        "camera:disk:{}",
        folders.root.path().join("missing").display()
    );

    let output = import(&folders, &["cardA", &camera, "--destination", "library"]);

    assert_eq!(
        fs::read(library(&folders).join(DAY).join("a.jpg")).unwrap(),
        picture("a.jpg")
    );
    assert!(
        stderr(&output)
            .lines()
            .any(|line| line.starts_with(&format!("error: {camera}: "))),
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
            "--destination",
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
            "--destination",
            "library",
            "--template",
            "{job}/{name}.{ext}",
        ],
    );

    assert!(stderr(&output).contains("{job}"), "{}", stderr(&output));
    assert!(!library(&folders).exists());
    assert_eq!(output.status.code(), Some(2));
}

/// A profile `holiday` that names every file by its own name only.
fn holiday_profile(folders: &Folders) {
    folders.write_profile(
        "holiday",
        b"schema_version: 1\nnaming:\n  template: \"{name}.{ext}\"\n",
    );
}

/// Asserts that `library` holds `a.jpg` and `b.jpg` by the template of the profile.
fn assert_named_by_the_profile(folders: &Folders) {
    for name in ["a.jpg", "b.jpg"] {
        assert_eq!(
            fs::read(library(folders).join(name)).unwrap(),
            picture(name),
            "{name}"
        );
    }
}

/// spec: CLI-015.7
#[test]
fn with_a_profile_the_last_folder_is_still_the_destination() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);
    card(&folders, "cardB", &["b.jpg"]);
    holiday_profile(&folders);

    let output = import(
        &folders,
        &["cardA", "cardB", "library", "--profile", "holiday"],
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_named_by_the_profile(&folders);
    let card_b = fs::read_dir(folders.root.path().join("cardB")).unwrap();
    assert_eq!(card_b.count(), 1, "nothing is written to cardB");
}

/// spec: CLI-015.8
#[test]
fn with_destination_and_a_profile_every_folder_is_a_source() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);
    card(&folders, "cardB", &["b.jpg"]);
    holiday_profile(&folders);

    let output = import(
        &folders,
        &[
            "cardA",
            "cardB",
            "--destination",
            "library",
            "--profile",
            "holiday",
        ],
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_named_by_the_profile(&folders);
}

/// spec: CLI-015.9
#[test]
fn the_error_of_a_source_comes_before_the_report_of_the_next() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);
    // A camera on a port of a missing folder cannot be opened by any user, also root.
    let camera = format!(
        "camera:disk:{}",
        folders.root.path().join("missing").display()
    );
    let both = folders.root.path().join("output.txt");
    let file = fs::File::create(&both).unwrap();

    let status = folders
        .framehive_command(&["import", &camera, "cardA", "--destination", "library"])
        .current_dir(folders.root.path())
        .stdout(Stdio::from(file.try_clone().unwrap()))
        .stderr(Stdio::from(file))
        .status()
        .unwrap();

    let written = fs::read_to_string(&both).unwrap();
    let position = |wanted: &dyn Fn(&str) -> bool| written.lines().position(wanted);
    let error = position(&|line| line.starts_with(&format!("error: {camera}: ")));
    let report = position(&|line| line == "cardA:");
    assert!(error.is_some() && report.is_some(), "{written}");
    assert!(error < report, "{written}");
    assert_eq!(status.code(), Some(2));
}

/// spec: CLI-015.10
#[test]
fn a_dry_run_of_several_sources_counts_across_them() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg", "b.jpg"]);
    card(&folders, "cardB", &["c.jpg"]);
    let stored = "format_version=1\nstored_last=0\n";
    folders.write_counters(stored);

    let output = import(
        &folders,
        &[
            "cardA",
            "cardB",
            "--destination",
            "library",
            "--dry-run",
            "--template",
            "{seq_stored:3}.{ext}",
        ],
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let mut first_card: Vec<_> = lines(&output)
        .into_iter()
        .filter(|line| {
            line.starts_with("would copy  a.jpg") || line.starts_with("would copy  b.jpg")
        })
        .map(|line| line.rsplit(' ').next().unwrap().to_owned())
        .collect();
    first_card.sort();
    assert_eq!(first_card, ["001.jpg", "002.jpg"], "{}", stdout(&output));
    assert!(
        has_line(&output, "would copy  c.jpg -> 003.jpg"),
        "{}",
        stdout(&output)
    );
    assert_eq!(fs::read_to_string(folders.counter_file()).unwrap(), stored);
    assert!(!library(&folders).exists());
}
