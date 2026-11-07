// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The command line of `framehive` (CLI-001): help, version and invalid arguments,
//! against the real binary.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::process::{Command, Output};

fn framehive(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_framehive"))
        .args(arguments)
        .env("TZ", "UTC")
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn version_prints_the_program_name_and_version() {
    let output = framehive(&["--version"]);

    assert_eq!(
        text(&output.stdout),
        format!("framehive {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn help_describes_the_import_arguments() {
    let output = framehive(&["import", "--help"]);

    let help = text(&output.stdout);
    for argument in [
        // The last folder is the destination, unless --destination names it (CLI-016).
        "framehive import [OPTIONS] <source>... <destination>",
        "framehive import [OPTIONS] --destination <folder> <source>...",
        "then the folder to import into",
        "--profile <name>",
        "--template",
        "--job-code",
        "--on-conflict",
        "--dry-run",
    ] {
        assert!(help.contains(argument), "{argument} missing in:\n{help}");
    }
    assert_eq!(output.status.code(), Some(0));
}

/// spec: CLI-001.11
#[test]
fn an_unknown_conflict_policy_is_rejected() {
    let output = framehive(&[
        "import",
        "/source",
        "--destination",
        "/library",
        "--on-conflict",
        "overwrite",
    ]);

    let error = text(&output.stderr);
    assert!(error.contains("overwrite"), "{error}");
    assert!(error.contains("--on-conflict"), "{error}");
    assert!(error.contains("--help"), "{error}");
    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-016.1
#[test]
fn a_command_without_a_source_shows_an_example() {
    let output = framehive(&["import"]);

    assert_eq!(
        text(&output.stderr),
        "error: no source: give the folders or cameras to import from, then the folder to import into\n\
         hint: framehive import /media/card ~/Pictures\n"
    );
    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-016.1
#[test]
fn a_destination_without_a_source_shows_an_example_with_it() {
    let output = framehive(&["import", "--destination", "/library"]);

    assert_eq!(
        text(&output.stderr),
        "error: no source: give the folders or cameras to import from\n\
         hint: framehive import --destination /library /media/card\n"
    );
    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-016.2
#[test]
fn a_single_folder_needs_a_destination() {
    let output = framehive(&["import", "/nowhere/card"]);

    assert_eq!(
        text(&output.stderr),
        "error: no destination: give the folder to import into after /nowhere/card, or with --destination\n\
         hint: framehive import /nowhere/card ~/Pictures\n"
    );
    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-016.12
#[test]
fn a_camera_cannot_be_the_destination() {
    let output = framehive(&["import", "/cardA", "camera:X-T5"]);

    assert_eq!(
        text(&output.stderr),
        "error: camera:X-T5 cannot be the destination; give the folder to import into last, or with --destination\n"
    );
    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-001.11
#[test]
fn an_unknown_subcommand_is_rejected() {
    let output = framehive(&["export", "/library"]);

    let error = text(&output.stderr);
    assert!(error.contains("export"), "{error}");
    assert!(error.contains("--help"), "{error}");
    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(2));
}
