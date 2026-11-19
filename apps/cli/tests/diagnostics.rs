// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The diagnostic output of `--verbose` (CLI-021) in a real import.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use support::{Folders, stderr, stdout};

/// spec: CLI-021.1
#[test]
fn an_import_without_verbose_writes_no_diagnostic_lines() {
    let folders = Folders::new();
    folders.add_raf();

    let output = folders.import(&[]);

    assert_eq!(output.status.code(), Some(0));
    assert!(!stderr(&output).contains(" INFO "), "{}", stderr(&output));
}

/// spec: CLI-021.4
#[test]
fn a_verbose_import_names_the_job_and_each_file_on_standard_error() {
    let folders = Folders::new();
    folders.add_raf();

    let output = folders.import(&["--verbose"]);

    let diagnostics = stderr(&output);
    let source = folders.source().display().to_string();
    assert_eq!(output.status.code(), Some(0));
    assert!(
        diagnostics.lines().any(|line| line.starts_with(" INFO ")
            && line.contains(&format!(
                "import_job{{source={source}}}:file{{path=DSCF0001.RAF}}"
            ))
            && line.contains("file done")
            && line.contains("outcome=Copied")),
        "{diagnostics}"
    );
    assert!(
        diagnostics.contains("import job done end=Finished"),
        "{diagnostics}"
    );
    assert!(!stdout(&output).contains(" INFO "), "{}", stdout(&output));
}

/// spec: CLI-021.5
#[test]
fn verbose_is_accepted_before_the_subcommand_and_as_v() {
    let folders = Folders::new();

    let output = folders.framehive(&["-v", "import-log"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}
