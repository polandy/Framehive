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
        "<source>",
        "<destination>",
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

/// spec: CLI-001.11
#[test]
fn a_missing_destination_is_rejected() {
    let output = framehive(&["import", "/source"]);

    let error = text(&output.stderr);
    assert!(error.contains("<destination>"), "{error}");
    assert!(error.contains("--help"), "{error}");
    assert_eq!(output.status.code(), Some(2));
}
