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
        // Every folder is a source; the destination is named (CLI-001, CLI-015).
        "<source>... [--destination <folder>]",
        // A profile can name the destination (CLI-010).
        "the destination of --profile",
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
    let output = framehive(&["import", "--destination", "/library"]);

    assert_eq!(
        text(&output.stderr),
        "error: no source: give the folders or cameras to import from\n\
         hint: framehive import /media/card --destination ~/Pictures\n"
    );
    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-016.4
#[test]
fn a_last_folder_without_destination_shows_how_to_name_it() {
    let output = framehive(&["import", "/cardA", "/cardB", "/library"]);

    assert_eq!(
        text(&output.stderr),
        "error: no destination: give --destination <folder>\n\
         hint: every folder is a source; to import into /library, write: \
         framehive import /cardA /cardB --destination /library\n"
    );
    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-016.5
#[test]
fn a_camera_last_gives_no_hint() {
    let output = framehive(&["import", "/cardA", "camera:"]);

    assert_eq!(
        text(&output.stderr),
        "error: no destination: give --destination <folder>\n"
    );
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
