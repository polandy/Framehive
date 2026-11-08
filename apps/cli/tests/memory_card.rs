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
use support::{Folders, has_line, photo, stderr, stdout, summary_line};

const FOLDER: &str = "DCIM/100_FUJI";
const DAY: &str = "2026/2026-10-17";
/// A video whose target the destination already holds with other content.
const TAKEN: &str = "DSCF0003.MOV";

/// What a folder holds: each entry's path relative to the folder, with its modification
/// time and, for a file, its content.
type Snapshot = BTreeMap<PathBuf, (SystemTime, Option<Vec<u8>>)>;

/// A memory card with a RAW+JPEG pair, two videos and a hidden folder.
fn memory_card(folders: &Folders) {
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    folders.add("DCIM/100_FUJI/DSCF0001.JPG", &photo());
    folders.add("DCIM/100_FUJI/DSCF0002.MOV", b"a video");
    folders.add(&format!("{FOLDER}/{TAKEN}"), b"another video");
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
    // A name conflict fails the file for every user; root reads a file regardless of
    // its mode.
    let taken = folders.destination().join(DAY).join(TAKEN);
    fs::create_dir_all(taken.parent().unwrap()).unwrap();
    fs::write(&taken, b"a video imported elsewhere").unwrap();
    let fail = ["--on-conflict", "fail"];

    let dry_run = folders.import(&["--dry-run", fail[0], fail[1]]);
    let first = folders.import(&fail);
    let destination_after_first = snapshot(&folders.destination());
    let second = folders.import(&fail);
    let destination_after_second = snapshot(&folders.destination());

    // A dry run plans every file, foresees the name conflict, and writes nothing.
    assert_has_line(
        &dry_run,
        &copied_line("DSCF0001.RAF", false).replacen("copied", "would copy", 1),
    );
    assert_has_line(&dry_run, "left out  .Trashes: hidden");
    assert_has_line(
        &dry_run,
        &format!("failed  {FOLDER}/{TAKEN}: name conflict"),
    );
    assert_eq!(dry_run.status.code(), Some(1), "{}", stderr(&dry_run));

    // The import copies the pair and the video, reports the hidden folder, and fails
    // only the video whose target is taken.
    assert_has_line(&first, &copied_line("DSCF0001.RAF", false));
    assert_has_line(&first, &copied_line("DSCF0001.JPG", false));
    assert_has_line(&first, &copied_line("DSCF0002.MOV", true));
    assert_has_line(&first, "left out  .Trashes: hidden");
    assert_has_line(&first, &format!("failed  {FOLDER}/{TAKEN}: name conflict"));
    assert_eq!(
        summary_line(&first),
        "3 copied, 0 already imported, 0 duplicates, 0 skipped, 1 failed, 1 left out"
    );
    assert_eq!(first.status.code(), Some(1), "{}", stderr(&first));
    for name in ["DSCF0001.RAF", "DSCF0001.JPG", "DSCF0002.MOV"] {
        let source = &source_before[&PathBuf::from(FOLDER).join(name)];
        let target = &destination_after_first[&PathBuf::from(DAY).join(name)];
        assert_eq!(target, source, "{name} differs from its source");
    }

    // The second import finds everything imported before and changes nothing.
    assert_has_line(
        &second,
        &copied_line("DSCF0001.RAF", false).replacen("copied", "imported before", 1),
    );
    assert_eq!(
        summary_line(&second),
        "0 copied, 3 already imported, 0 duplicates, 0 skipped, 1 failed, 1 left out"
    );
    assert_eq!(second.status.code(), Some(1), "{}", stderr(&second));
    assert_eq!(destination_after_second, destination_after_first);

    // The source is never moved or changed.
    assert_eq!(snapshot(&folders.source()), source_before);
}
