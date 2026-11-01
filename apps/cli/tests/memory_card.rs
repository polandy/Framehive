// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The guarantees of milestone 1 end to end: a dry run, an import and a second import
//! of a folder laid out like a camera memory card, against the real binary and the real
//! file system. The child process runs with `TZ=UTC`, so local time is UTC.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::SystemTime;

use metadata::testing::raf;
use support::{Folders, has_line, lines, photo, set_mode, stderr, stdout};

const FOLDER: &str = "DCIM/100_FUJI";
const DAY: &str = "2026/2026-10-17";
const UNREADABLE: &str = "DCIM/100_FUJI/DSCF0003.MOV";

/// What a folder holds: each entry's path relative to the folder, with its modification
/// time and, for a file, its content.
type Snapshot = BTreeMap<PathBuf, (SystemTime, Option<Vec<u8>>)>;

/// A memory card with a RAW+JPEG pair, a video, a hidden folder and a video that
/// cannot be read (unless the test runs as root, which reads it regardless of its mode).
fn memory_card(folders: &Folders) {
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    folders.add("DCIM/100_FUJI/DSCF0001.JPG", &photo());
    folders.add("DCIM/100_FUJI/DSCF0002.MOV", b"a video");
    folders.add(UNREADABLE, b"another video");
    folders.add(".Trashes/DSCF0009.RAF", &raf(&photo()));
}

fn snapshot(root: &Path) -> Snapshot {
    let mut entries = Snapshot::new();
    add_to_snapshot(root, root, &mut entries);
    entries
}

fn add_to_snapshot(root: &Path, folder: &Path, entries: &mut Snapshot) {
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        let metadata = fs::symlink_metadata(&path).unwrap();
        let content = metadata.is_file().then(|| fs::read(&path).unwrap());
        if metadata.is_dir() {
            add_to_snapshot(root, &path, entries);
        }
        let relative = path.strip_prefix(root).unwrap().to_path_buf();
        entries.insert(relative, (metadata.modified().unwrap(), content));
    }
}

fn copied_line(name: &str, dated_by_modification_time: bool) -> String {
    let line = format!("copied  {FOLDER}/{name} -> {DAY}/{name}");
    if dated_by_modification_time {
        format!("{line} (dated by modification time: format not read)")
    } else {
        line
    }
}

fn summary(output: &Output) -> String {
    lines(output).last().cloned().unwrap_or_default()
}

fn assert_has_line(output: &Output, line: &str) {
    assert!(
        has_line(output, line),
        "missing `{line}` in:\n{}{}",
        stdout(output),
        stderr(output)
    );
}

#[test]
fn a_memory_card_is_copied_once_and_left_unchanged() {
    let folders = Folders::new();
    memory_card(&folders);
    let source_before = snapshot(&folders.source());
    let unreadable = folders.source().join(UNREADABLE);
    set_mode(&unreadable, 0o000);
    let can_be_read = fs::read(&unreadable).is_ok();

    let dry_run = folders.import(&["--dry-run"]);
    let first = folders.import(&[]);
    let destination_after_first = snapshot(&folders.destination());
    let second = folders.import(&[]);
    let destination_after_second = snapshot(&folders.destination());
    set_mode(&unreadable, 0o644);

    // A dry run plans every file, and writes nothing.
    assert_has_line(
        &dry_run,
        &copied_line("DSCF0001.RAF", false).replacen("copied", "would copy", 1),
    );
    assert_has_line(&dry_run, "left out  .Trashes: hidden");
    assert_eq!(dry_run.status.code(), Some(0), "{}", stderr(&dry_run));

    // The import copies the pair and the video, reports the hidden folder, and fails
    // only the file it cannot read.
    assert_has_line(&first, &copied_line("DSCF0001.RAF", false));
    assert_has_line(&first, &copied_line("DSCF0001.JPG", false));
    assert_has_line(&first, &copied_line("DSCF0002.MOV", true));
    assert_has_line(&first, "left out  .Trashes: hidden");
    let (copied, failed, exit_code) = if can_be_read { (4, 0, 0) } else { (3, 1, 1) };
    assert_eq!(
        summary(&first),
        format!(
            "{copied} copied, 0 already imported, 0 duplicates, 0 skipped, \
             {failed} failed, 1 left out"
        )
    );
    assert_eq!(first.status.code(), Some(exit_code), "{}", stderr(&first));
    for name in ["DSCF0001.RAF", "DSCF0001.JPG", "DSCF0002.MOV"] {
        let source = &source_before[&PathBuf::from(FOLDER).join(name)];
        let target = &destination_after_first[&PathBuf::from(DAY).join(name)];
        assert_eq!(target, source, "{name} differs from its source");
    }

    // The second import finds everything already imported and changes nothing.
    assert_has_line(
        &second,
        &copied_line("DSCF0001.RAF", false).replacen("copied", "already imported", 1),
    );
    assert_eq!(
        summary(&second),
        format!(
            "0 copied, {copied} already imported, 0 duplicates, 0 skipped, \
             {failed} failed, 1 left out"
        )
    );
    assert_eq!(second.status.code(), Some(exit_code), "{}", stderr(&second));
    assert_eq!(destination_after_second, destination_after_first);

    // The source is never moved or changed.
    assert_eq!(snapshot(&folders.source()), source_before);
}
