// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive library keep-raw --dry-run` against the real binary (CLI-023.2, CLI-023.3).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use support::{Folders, stderr, stdout};

/// spec: CLI-023.2
#[test]
fn keep_raw_dry_run_changes_nothing() {
    let folders = Folders::new();
    folders.add("A.RAF", b"raw");
    let jpeg = folders.add("A.JPG", b"jpeg");

    let output = folders.framehive(&[
        "library",
        "keep-raw",
        &jpeg.display().to_string(),
        "--dry-run",
    ]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let asset = folders.source().join("A");
    assert_eq!(
        stdout(&output),
        format!("Keep RAW would be on: {}\n", asset.display())
    );
    assert!(!folders.source().join("A.RAF.xmp").exists());
    assert!(!folders.source().join("A.JPG.xmp").exists());
}

/// spec: CLI-023.3
#[test]
fn keep_raw_dry_run_checks_the_files() {
    let folders = Folders::new();
    let raw = folders.add("A.RAF", b"raw");
    let notes = folders.add("notes.txt", b"notes");

    let output = folders.framehive(&[
        "library",
        "keep-raw",
        &raw.display().to_string(),
        &notes.display().to_string(),
        "--dry-run",
    ]);

    assert_eq!(output.status.code(), Some(2));
    let error = format!(
        "error: {} is not a photo or video of the library",
        notes.display()
    );
    assert!(stderr(&output).contains(&error), "{}", stderr(&output));
    assert!(stdout(&output).is_empty());
    assert!(!folders.source().join("A.RAF.xmp").exists());
}
