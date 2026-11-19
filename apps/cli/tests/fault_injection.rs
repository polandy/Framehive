// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The binary built with the fault-injection point (ADR 0048), run with real adapters.
//! It compiles only with the feature `fault-injection`, which the CI step
//! `fault injection` sets.

#![cfg(feature = "fault-injection")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::process::Output;

use metadata::testing::raf;
use support::{
    DEFAULT_TARGET, Folders, has_line, has_line_starting_with, lines, photo, picture, stderr,
    summary_line,
};

const NAMES: [&str; 3] = ["DSCF0001.RAF", "DSCF0002.RAF", "DSCF0003.RAF"];
const SIGABRT: i32 = 6;

/// Adds the three files of [`NAMES`] to the source, each with its own content.
fn add_three_files(folders: &Folders) {
    for name in NAMES {
        folders.add(name, &raf(&picture(name)));
    }
}

fn target(name: &str) -> String {
    format!("2026/2026-10-17/{name}")
}

fn copied(name: &str) -> String {
    format!("copied  {name} -> {}", target(name))
}

fn imported_before(name: &str) -> String {
    format!("imported before  {name} -> {}", target(name))
}

/// Runs `framehive import <source> --destination <destination>` with `fault` in
/// `FRAMEHIVE_FAULT`.
fn import_with_fault(folders: &Folders, fault: &str) -> Output {
    let source = folders.source().display().to_string();
    let destination = folders.destination().display().to_string();
    folders
        .framehive_command(&["import", &source, "--destination", &destination])
        .env("FRAMEHIVE_FAULT", fault)
        .output()
        .unwrap()
}

fn assert_target_is_source(folders: &Folders, name: &str) {
    assert_eq!(
        fs::read(folders.destination().join(target(name))).unwrap(),
        raf(&picture(name)),
        "{name}"
    );
}

/// Without a fault, the import of a binary with the fault-injection point copies as
/// the released binary does.
/// spec: CLI-001.1
#[test]
fn imports_as_usual_without_a_fault() {
    let folders = Folders::new();
    folders.add_raf();

    let output = folders.import(&[]);

    assert_eq!(
        summary_line(&output),
        "1 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out"
    );
    assert_eq!(
        fs::read(folders.destination().join(DEFAULT_TARGET)).unwrap(),
        raf(&photo())
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-025.1
#[test]
fn a_killed_import_leaves_only_a_part_file_and_resumes() {
    let folders = Folders::new();
    add_three_files(&folders);

    let killed = import_with_fault(&folders, "abort-while-writing DSCF0002.RAF");

    assert_eq!(killed.status.signal(), Some(SIGABRT), "{}", stderr(&killed));
    assert_eq!(lines(&killed), [copied("DSCF0001.RAF")]);
    assert_target_is_source(&folders, "DSCF0001.RAF");
    let part = folders.destination().join(target("DSCF0002.RAF.part"));
    let written = raf(&picture("DSCF0002.RAF"));
    assert_eq!(fs::read(&part).unwrap(), written[..written.len() / 2]);
    assert!(!folders.destination().join(target("DSCF0002.RAF")).exists());
    assert!(!folders.destination().join(target("DSCF0003.RAF")).exists());

    let resumed = folders.import(&[]);

    assert!(has_line(&resumed, &imported_before("DSCF0001.RAF")));
    assert!(has_line(&resumed, &copied("DSCF0002.RAF")));
    assert!(has_line(&resumed, &copied("DSCF0003.RAF")));
    assert_eq!(
        summary_line(&resumed),
        "2 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out"
    );
    for name in NAMES {
        assert_target_is_source(&folders, name);
    }
    assert!(!part.exists());
    assert_eq!(resumed.status.code(), Some(0), "{}", stderr(&resumed));
}

/// spec: CLI-025.2
#[test]
fn a_bit_error_at_the_destination_fails_the_file_and_a_rerun_copies_it() {
    let folders = Folders::new();
    add_three_files(&folders);

    let output = import_with_fault(&folders, "flip-bit-after-sync DSCF0002.RAF");

    assert!(has_line(&output, &copied("DSCF0001.RAF")));
    assert!(has_line_starting_with(
        &output,
        "failed  DSCF0002.RAF: cannot verify the target: the target has the checksum blake3:"
    ));
    assert!(has_line(&output, &copied("DSCF0003.RAF")));
    assert_eq!(
        summary_line(&output),
        "2 copied, 0 already imported, 0 duplicates, 0 skipped, 1 failed, 0 left out"
    );
    assert!(has_line(
        &output,
        "Do not format the source: not every file is verified in 1 location."
    ));
    assert!(has_line(
        &output,
        "  DSCF0002.RAF: verified in 0 of 1 location"
    ));
    assert!(!folders.destination().join(target("DSCF0002.RAF")).exists());
    assert!(
        !folders
            .destination()
            .join(target("DSCF0002.RAF.part"))
            .exists()
    );
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));

    let rerun = folders.import(&[]);

    assert!(has_line(&rerun, &copied("DSCF0002.RAF")));
    assert_eq!(
        summary_line(&rerun),
        "1 copied, 2 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out"
    );
    assert_target_is_source(&folders, "DSCF0002.RAF");
    assert_eq!(rerun.status.code(), Some(0), "{}", stderr(&rerun));
}

/// spec: CLI-025.3
#[test]
fn an_unplugged_source_fails_the_remaining_files_and_a_rerun_copies_them() {
    let folders = Folders::new();
    add_three_files(&folders);

    let output = import_with_fault(&folders, "unplug-while-reading DSCF0002.RAF");

    assert!(has_line(&output, &copied("DSCF0001.RAF")));
    for name in ["DSCF0002.RAF", "DSCF0003.RAF"] {
        let failed = lines(&output)
            .into_iter()
            .find(|line| line.starts_with(&format!("failed  {name}: cannot read the source: ")))
            .unwrap_or_default();
        assert!(
            failed.ends_with("Input/output error (os error 5)"),
            "{failed}"
        );
        assert!(has_line(
            &output,
            &format!("  {name}: verified in 0 of 1 location")
        ));
        assert!(!folders.destination().join(target(name)).exists());
        assert!(
            !folders
                .destination()
                .join(target(&format!("{name}.part")))
                .exists()
        );
    }
    assert_eq!(
        summary_line(&output),
        "1 copied, 0 already imported, 0 duplicates, 0 skipped, 2 failed, 0 left out"
    );
    assert!(has_line(
        &output,
        "Do not format the source: not every file is verified in 1 location."
    ));
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));

    let rerun = folders.import(&[]);

    assert!(has_line(&rerun, &imported_before("DSCF0001.RAF")));
    assert_eq!(
        summary_line(&rerun),
        "2 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out"
    );
    for name in NAMES {
        assert_target_is_source(&folders, name);
    }
    assert!(has_line(
        &rerun,
        "The source can be formatted: every file is verified in 1 location."
    ));
    assert_eq!(rerun.status.code(), Some(0), "{}", stderr(&rerun));
}

/// spec: CLI-025.4
#[test]
fn an_unknown_fault_is_a_usage_error() {
    let folders = Folders::new();
    add_three_files(&folders);

    let output = import_with_fault(&folders, "flip-bits DSCF0002.RAF");

    assert_eq!(
        stderr(&output),
        "error: FRAMEHIVE_FAULT: unknown fault `flip-bits`; the faults are \
         abort-while-writing, flip-bit-after-sync, unplug-while-reading\n"
    );
    assert!(!folders.destination().exists());
    assert_eq!(output.status.code(), Some(2));
}
