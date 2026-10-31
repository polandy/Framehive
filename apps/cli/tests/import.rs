// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` (CLI-001) runs that start, against the real binary, the real file
//! system and a temporary folder. The child process runs with `TZ=UTC`, so local time
//! is UTC.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use metadata::testing::raf;
use support::{
    COPIED_LINE, DEFAULT_TARGET, Folders, has_line, has_line_starting_with, lines, photo, set_mode,
    stderr, stdout,
};

/// spec: CLI-001.1
#[test]
fn imports_with_the_defaults() {
    let folders = Folders::new();
    folders.add_raf();

    let output = folders.import(&[]);

    assert_eq!(
        stdout(&output),
        format!(
            "{COPIED_LINE}\n\
             1 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n"
        )
    );
    assert_eq!(
        fs::read(folders.destination().join(DEFAULT_TARGET)).unwrap(),
        raf(&photo())
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-001.2
#[test]
fn a_second_run_copies_nothing() {
    let folders = Folders::new();
    folders.add_raf();
    folders.import(&[]);
    let target = folders.destination().join(DEFAULT_TARGET);
    let modified_before = fs::metadata(&target).unwrap().modified().unwrap();

    let output = folders.import(&[]);

    assert!(
        has_line(
            &output,
            "already imported  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF"
        ),
        "{}",
        stdout(&output)
    );
    assert_eq!(fs::read(&target).unwrap(), raf(&photo()));
    assert_eq!(
        fs::metadata(&target).unwrap().modified().unwrap(),
        modified_before
    );
    assert_eq!(output.status.code(), Some(0));
}

/// spec: CLI-001.3
#[test]
fn uses_an_own_template() {
    let folders = Folders::new();
    folders.add_raf();

    let output = folders.import(&["--template", "{date:%Y%m%d}-{name}.{ext}"]);

    assert!(
        folders
            .destination()
            .join("20261017-DSCF0001.RAF")
            .is_file()
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-001.4
#[test]
fn follows_the_conflict_policy() {
    let folders = Folders::new();
    folders.add_raf();
    let existing = folders.destination().join(DEFAULT_TARGET);
    fs::create_dir_all(existing.parent().unwrap()).unwrap();
    fs::write(&existing, b"another photo").unwrap();

    let skip = folders.import(&["--on-conflict", "skip"]);
    let fail = folders.import(&["--on-conflict", "fail"]);

    assert!(
        has_line(&skip, "skipped  DSCF0001.RAF: name conflict"),
        "{}",
        stdout(&skip)
    );
    assert_eq!(skip.status.code(), Some(0));
    assert!(
        has_line(&fail, "failed  DSCF0001.RAF: name conflict"),
        "{}",
        stdout(&fail)
    );
    assert_eq!(fail.status.code(), Some(1));
    assert_eq!(fs::read(&existing).unwrap(), b"another photo");
}

/// spec: CLI-001.5
#[test]
fn a_dry_run_copies_nothing() {
    let folders = Folders::new();
    folders.add_raf();

    let output = folders.import(&["--dry-run"]);

    assert!(
        has_line(
            &output,
            "would copy  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF"
        ),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(!folders.destination().exists());
    assert_eq!(output.status.code(), Some(0));
}

/// spec: CLI-001.6
#[test]
fn a_file_that_fails_does_not_stop_the_import() {
    let folders = Folders::new();
    for name in ["a.jpg", "b.jpg", "c.jpg"] {
        folders.add(name, &photo());
    }
    let unreadable = folders.source().join("b.jpg");
    set_mode(&unreadable, 0o000);
    // Root reads the file regardless of its mode; then it is imported like the others.
    let can_be_read = fs::read(&unreadable).is_ok();

    let output = folders.import(&[]);
    set_mode(&unreadable, 0o644);

    let day = folders.destination().join("2026/2026-10-17");
    assert!(day.join("a.jpg").is_file());
    assert!(day.join("c.jpg").is_file());
    if can_be_read {
        assert!(day.join("b.jpg").is_file());
        assert_eq!(output.status.code(), Some(0));
    } else {
        assert!(
            has_line_starting_with(&output, "failed  b.jpg: "),
            "{}",
            stdout(&output)
        );
        let summary = lines(&output).last().cloned().unwrap_or_default();
        assert!(summary.starts_with("2 copied,"), "{summary}");
        assert!(summary.contains(" 1 failed,"), "{summary}");
        assert_eq!(output.status.code(), Some(1));
    }
}

/// spec: CLI-001.7
#[test]
fn reports_files_left_out_and_dated_by_modification_time() {
    let folders = Folders::new();
    folders.add("notes.txt", b"notes");
    folders.add("clip.MOV", b"a video");

    let output = folders.import(&[]);

    assert!(
        has_line(&output, "left out  notes.txt: unsupported type"),
        "{}",
        stdout(&output)
    );
    assert!(
        has_line(
            &output,
            "copied  clip.MOV -> 2026/2026-10-17/clip.MOV \
             (dated by modification time: format not read)"
        ),
        "{}",
        stdout(&output)
    );
    assert_eq!(output.status.code(), Some(0));
}

/// spec: CLI-001.12
#[test]
fn creates_a_missing_destination_with_its_parents() {
    let folders = Folders::new();
    folders.add_raf();
    let destination = folders.root.path().join("new/library");

    let output = folders.import_into(&destination, &[]);

    assert!(destination.join(DEFAULT_TARGET).is_file());
    assert!(has_line(&output, COPIED_LINE), "{}", stdout(&output));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-001.13
#[test]
fn reports_a_folder_that_cannot_be_read() {
    let folders = Folders::new();
    folders.add_raf();
    folders.add("locked/DSCF0002.RAF", &raf(&photo()));
    let locked = folders.source().join("locked");
    set_mode(&locked, 0o000);
    // Root reads the folder regardless of its mode; then it is imported as well.
    let can_be_read = fs::read_dir(&locked).is_ok();

    let output = folders.import(&[]);
    set_mode(&locked, 0o755);

    assert!(has_line(&output, COPIED_LINE), "{}", stdout(&output));
    if can_be_read {
        assert_eq!(output.status.code(), Some(0));
    } else {
        assert!(
            has_line_starting_with(&output, "left out  locked: "),
            "{}",
            stdout(&output)
        );
        assert_eq!(output.status.code(), Some(1));
    }
}
