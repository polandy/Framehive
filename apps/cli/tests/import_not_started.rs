// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` (CLI-001) runs that do not start: exit code 2, the reason on
//! standard error, and nothing written.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use support::{Folders, stderr};

/// spec: CLI-001.8
#[test]
fn rejects_an_invalid_template() {
    let folders = Folders::new();
    folders.add_raf();

    let output = folders.import(&["--template", "{date:%Y}/{camera}.{ext}"]);

    let error = stderr(&output);
    assert!(error.contains("{camera}"), "{error}");
    assert!(error.contains("column 11"), "{error}");
    assert!(!folders.destination().exists());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-001.9
#[test]
fn rejects_a_missing_source() {
    let folders = Folders::new();
    fs::remove_dir(folders.source()).unwrap();

    let output = folders.import(&[]);

    let error = stderr(&output);
    assert!(
        error.contains(&folders.source().display().to_string()),
        "{error}"
    );
    assert!(!folders.destination().exists());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-001.10
#[test]
fn rejects_a_destination_inside_the_source() {
    let folders = Folders::new();
    folders.add_raf();
    let destination = folders.source().join("library");

    let output = folders.import_into(&destination, &[]);

    // The command names the paths without symbolic links, e.g. /private/tmp on macOS.
    let source = fs::canonicalize(folders.source()).unwrap();
    let error = stderr(&output);
    assert!(error.contains(&source.display().to_string()), "{error}");
    assert!(
        error.contains(&source.join("library").display().to_string()),
        "{error}"
    );
    assert!(!destination.exists());
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn rejects_a_destination_that_cannot_be_created() {
    let folders = Folders::new();
    folders.add_raf();
    let file = folders.root.path().join("file");
    fs::write(&file, b"not a folder").unwrap();

    let output = folders.import_into(&file.join("library"), &[]);

    assert!(stderr(&output).contains("library"), "{}", stderr(&output));
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn rejects_parent_folder_steps_in_a_missing_destination() {
    let folders = Folders::new();
    folders.add_raf();
    let destination = folders.root.path().join("new/../library");

    let output = folders.import_into(&destination, &[]);

    assert!(stderr(&output).contains(".."), "{}", stderr(&output));
    assert!(!folders.root.path().join("new").exists());
    assert_eq!(output.status.code(), Some(2));
}
