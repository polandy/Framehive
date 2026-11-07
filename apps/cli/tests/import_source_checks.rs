// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The checks of every source before any job of `framehive import` (CLI-016). The
//! sources are folders below the test's root, named relative to it.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::{Path, PathBuf};
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

/// The names of the files below `folder`, sorted.
fn file_names_below(folder: &Path) -> Vec<String> {
    let mut names = Vec::new();
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            names.extend(file_names_below(&path));
        } else {
            names.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    names.sort();
    names
}

/// spec: CLI-016.14
#[test]
fn the_last_folder_is_the_destination() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);
    card(&folders, "cardB", &["b.jpg"]);

    let output = import(&folders, &["cardA", "cardB", "library"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(file_names_below(&library(&folders)), ["a.jpg", "b.jpg"]);
    let card_b = folders.root.path().join("cardB");
    assert_eq!(file_names_below(&card_b), ["b.jpg"]);
}

/// spec: CLI-016.14
#[test]
fn with_destination_every_folder_is_a_source() {
    let folders = Folders::new();
    card(&folders, "cardA", &["a.jpg"]);
    card(&folders, "cardB", &["b.jpg"]);

    let output = import(&folders, &["--destination", "library", "cardA", "cardB"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(file_names_below(&library(&folders)), ["a.jpg", "b.jpg"]);
}

/// spec: CLI-016.15
#[test]
fn the_arguments_are_checked_before_the_profile() {
    let folders = Folders::new();
    card(&folders, "card", &["a.jpg"]);

    let output = import(&folders, &["card", "--profile", "missing"]);

    assert_stopped_before_any_job(
        &folders,
        &output,
        "error: no destination: give the folder to import into after card, or with --destination\n\
         hint: framehive import card ~/Pictures",
    );
}

/// spec: CLI-016.15
#[test]
fn the_profile_is_checked_before_the_folders() {
    let folders = Folders::new();
    let profile = folders.write_profile("holiday", b"schema_version: 1\ndestination: /a\n");

    let output = import(&folders, &["nowhere", "library", "--profile", "holiday"]);

    assert_stopped_before_any_job(
        &folders,
        &output,
        &format!(
            "error: {}:2:1: destination is not read from a profile; give the folder to import into on the command line",
            profile.display()
        ),
    );
}

/// spec: CLI-016.2
/// spec: CLI-010.17
#[test]
fn a_profile_does_not_supply_the_destination() {
    let folders = Folders::new();
    card(&folders, "card", &["a.jpg"]);
    folders.write_profile(
        "holiday",
        b"schema_version: 1\nnaming:\n  template: \"{name}.{ext}\"\n",
    );

    let output = import(&folders, &["card", "--profile", "holiday"]);

    assert_stopped_before_any_job(
        &folders,
        &output,
        "error: no destination: give the folder to import into after card, or with --destination\n\
         hint: framehive import card ~/Pictures",
    );
}
