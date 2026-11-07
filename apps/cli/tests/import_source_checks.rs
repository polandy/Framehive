// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The checks of every source before any job of `framehive import` (CLI-016). The
//! sources are folders below the test's root, named relative to it.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::PathBuf;
use std::process::Output;

use support::{Folders, card, import_in_root as import, picture, stderr, stdout};

fn library(folders: &Folders) -> PathBuf {
    folders.root.path().join("library")
}

/// Asserts that the command stopped before any job with exactly `error`.
fn assert_stopped_before_any_job(folders: &Folders, output: &Output, error: &str) {
    assert_eq!(stderr(output), format!("{error}\n"));
    assert!(stdout(output).is_empty(), "{}", stdout(output));
    assert!(!library(folders).exists());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-016.6
#[test]
fn a_missing_source_starts_no_source() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);

    let output = import(&folders, &["cardA", "missing", "--destination", "library"]);

    assert_stopped_before_any_job(
        &folders,
        &output,
        "error: the source missing does not exist",
    );
}

/// spec: CLI-016.7
#[test]
fn a_file_as_source_starts_no_source() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);
    fs::write(folders.root.path().join("b.jpg"), picture("b.jpg")).unwrap();

    let output = import(&folders, &["cardA", "b.jpg", "--destination", "library"]);

    assert_stopped_before_any_job(
        &folders,
        &output,
        "error: the source b.jpg is not a folder; give the folder that holds it",
    );
}

/// spec: CLI-016.8
#[test]
fn a_destination_inside_a_later_source_starts_no_source() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);
    card(&folders, "cardB", &["b.jpg"]);
    let card_b = fs::canonicalize(folders.root.path().join("cardB")).unwrap();

    let output = import(
        &folders,
        &["cardA", "cardB", "--destination", "cardB/library"],
    );

    assert_eq!(
        stderr(&output),
        format!(
            "error: the destination {} lies inside the source {}; choose a destination outside it\n",
            card_b.join("library").display(),
            card_b.display()
        )
    );
    assert!(!card_b.join("library").exists());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-016.9
#[test]
fn a_source_inside_another_starts_no_source() {
    let folders = Folders::new();
    card(&folders, "card/DCIM", &["a.jpg"]);

    let output = import(&folders, &["card/DCIM", "card", "--destination", "library"]);

    assert_stopped_before_any_job(
        &folders,
        &output,
        "error: source card/DCIM lies inside the source card; give it once",
    );
}

/// spec: CLI-016.10
#[test]
fn a_source_given_twice_starts_no_source() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);

    let output = import(&folders, &["cardA", "./cardA", "--destination", "library"]);

    assert_stopped_before_any_job(
        &folders,
        &output,
        "error: source ./cardA is given twice; give it once",
    );
}

/// spec: CLI-016.11
#[test]
fn a_camera_given_twice_starts_no_source() {
    let folders = Folders::new();

    let output = import(
        &folders,
        &["camera:", "camera:", "--destination", "library"],
    );

    assert_stopped_before_any_job(
        &folders,
        &output,
        "error: source camera: is given twice; give it once",
    );
}
