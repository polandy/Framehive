// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The fault scenarios T4, T7, T8 and T9 of requirements section 9.2, run against the
//! binary with real adapters (CLI-022).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Duration, SystemTime};

use metadata::testing::{ByteOrder, Tiff, jpeg, raf};
use support::{
    Folders, MODIFIED, card, has_line, has_line_starting_with, import_in_root, lines, photo,
    picture, stderr,
};

const DAY: &str = "2026/2026-10-17";

fn last_line(output: &Output) -> String {
    lines(output).pop().unwrap_or_default()
}

/// The names of the files in `folder`, sorted.
fn file_names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn set_modified(path: &Path, seconds: u64) {
    let file = fs::File::options().write(true).open(path).unwrap();
    file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))
        .unwrap();
}

/// spec: CLI-022.1
#[test]
fn damaged_files_are_dated_by_modification_time_and_copied() {
    let folders = Folders::new();
    let whole = raf(&photo());
    let truncated = &whole[..whole.len() / 2];
    let broken_exif = jpeg(
        &Tiff::new(ByteOrder::LittleEndian)
            .with_exif_ifd_pointer(0xFFFF_FFF0)
            .to_bytes(),
    );
    folders.add("DSCF0001.RAF", &whole);
    folders.add("DSCF0002.RAF", truncated);
    folders.add("DSCF0003.JPG", &broken_exif);

    let output = folders.import(&[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    for (name, content) in [
        ("DSCF0001.RAF", whole.as_slice()),
        ("DSCF0002.RAF", truncated),
        ("DSCF0003.JPG", broken_exif.as_slice()),
    ] {
        let target = folders.destination().join(DAY).join(name);
        assert_eq!(fs::read(target).unwrap(), content, "{name}");
    }
    for name in ["DSCF0002.RAF", "DSCF0003.JPG"] {
        let line = lines(&output)
            .into_iter()
            .find(|line| line.starts_with(&format!("copied  {name} -> {DAY}/{name} ")))
            .unwrap();
        assert!(
            line.contains("(dated by modification time: damaged: "),
            "{line}"
        );
    }
    assert!(has_line(
        &output,
        "3 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out"
    ));
    assert_eq!(
        last_line(&output),
        "The source can be formatted: every file is verified in 1 location."
    );
}

/// A source with two pictures and a backup folder whose target folder is blocked by a
/// file `2026`, so no backup can be written.
fn blocked_backup() -> (Folders, PathBuf) {
    let folders = Folders::new();
    folders.add_raf();
    folders.add("DSCF0002.RAF", &raf(&picture("DSCF0002.RAF")));
    let backup = folders.root.path().join("backup");
    fs::create_dir(&backup).unwrap();
    fs::write(backup.join("2026"), b"in the way").unwrap();
    (folders, backup)
}

/// spec: CLI-022.2
#[test]
fn a_backup_that_cannot_be_written_keeps_the_card() {
    let (folders, backup) = blocked_backup();
    let shown = backup.display().to_string();

    let output = folders.import(&["--backup", &shown]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        file_names(&folders.destination().join(DAY)),
        ["DSCF0001.RAF", "DSCF0002.RAF"]
    );
    let printed = lines(&output);
    for name in ["DSCF0001.RAF", "DSCF0002.RAF"] {
        let copied = format!("copied  {name} -> {DAY}/{name}");
        let index = printed.iter().position(|line| *line == copied).unwrap();
        let next = &printed[index + 1];
        assert!(
            next.starts_with(&format!("  backup failed  {shown}: ")),
            "{next}"
        );
    }
    assert!(has_line(
        &output,
        &format!("backup {shown}: 0 backed up, 0 already backed up, 2 failed")
    ));
    assert_eq!(
        printed[printed.len() - 3..],
        [
            "Do not format the source: not every file is verified in 2 locations.",
            "  DSCF0001.RAF: verified in 1 of 2 locations",
            "  DSCF0002.RAF: verified in 1 of 2 locations",
        ]
    );
}

/// spec: CLI-022.3
#[test]
fn a_rerun_completes_the_backup_once_it_can_be_written() {
    let (folders, backup) = blocked_backup();
    let shown = backup.display().to_string();
    folders.import(&["--backup", &shown]);
    fs::remove_file(backup.join("2026")).unwrap();

    let output = folders.import(&["--backup", &shown]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    for name in ["DSCF0001.RAF", "DSCF0002.RAF"] {
        let reported = format!("imported before  {name} -> {DAY}/{name}");
        let index = lines(&output)
            .iter()
            .position(|line| *line == reported)
            .unwrap();
        assert_eq!(lines(&output)[index + 1], format!("  backed up  {shown}"));
    }
    assert_eq!(
        file_names(&backup.join(DAY)),
        ["DSCF0001.RAF", "DSCF0002.RAF"]
    );
    assert_eq!(
        last_line(&output),
        "The source can be formatted: every file is verified in 2 locations."
    );
}

/// spec: CLI-022.4
#[test]
fn the_same_file_on_both_card_slots_is_copied_once() {
    let folders = Folders::new();
    card(&folders, "slot1", &["DSCF0001.RAF"]);
    card(&folders, "slot2", &["DSCF0001.RAF"]);

    let output = import_in_root(&folders, &["slot1", "slot2", "--destination", "library"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    // The lines of the two jobs may interleave (CLI-015), so each summary is found by
    // its source line.
    let printed = lines(&output);
    let summary_of = |source: &str| {
        let index = printed.iter().position(|line| line == source).unwrap();
        printed[index + 1].clone()
    };
    assert_eq!(
        summary_of("slot1:"),
        "1 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out"
    );
    assert_eq!(
        summary_of("slot2:"),
        "0 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out"
    );
    assert!(has_line(
        &output,
        &format!("copied  DSCF0001.RAF -> {DAY}/DSCF0001.RAF")
    ));
    assert!(
        has_line(
            &output,
            &format!("already imported  DSCF0001.RAF -> {DAY}/DSCF0001.RAF")
        ) || has_line(
            &output,
            &format!("imported before  DSCF0001.RAF -> {DAY}/DSCF0001.RAF")
        ),
        "{printed:?}"
    );
    assert_eq!(
        file_names(&folders.root.path().join("library").join(DAY)),
        ["DSCF0001.RAF"]
    );
    let formattable = printed
        .iter()
        .filter(|line| {
            *line == "The source can be formatted: every file is verified in 1 location."
        })
        .count();
    assert_eq!(formattable, 2);
}

/// spec: CLI-022.5
#[test]
fn a_shifted_modification_time_is_still_imported_before() {
    let folders = Folders::new();
    let clip = folders.add("clip.MOV", b"movie");
    folders.import(&[]);
    set_modified(&clip, MODIFIED + 3600);

    let output = folders.import(&[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(has_line(
        &output,
        &format!("imported before  clip.MOV -> {DAY}/clip.MOV")
    ));
    assert!(has_line(
        &output,
        "0 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out"
    ));
    assert_eq!(file_names(&folders.destination().join(DAY)), ["clip.MOV"]);
}

/// spec: CLI-022.6
#[test]
fn an_overflowed_file_number_gets_a_name_of_its_own() {
    let folders = Folders::new();
    let names = [
        "DCIM/100_FUJI/DSCF0001.RAF",
        "DCIM/100_FUJI/DSCF9999.RAF",
        "DCIM/101_FUJI/DSCF0001.RAF",
    ];
    for name in names {
        folders.add(name, &raf(&picture(name)));
    }

    let first = folders.import(&[]);
    let second = folders.import(&[]);

    assert_eq!(first.status.code(), Some(0), "{}", stderr(&first));
    assert!(has_line(
        &first,
        &format!("copied  DCIM/100_FUJI/DSCF0001.RAF -> {DAY}/DSCF0001.RAF")
    ));
    assert!(has_line(
        &first,
        &format!("copied  DCIM/101_FUJI/DSCF0001.RAF -> {DAY}/DSCF0001-1.RAF")
    ));
    let library = folders.destination().join(DAY);
    for (name, target) in names
        .iter()
        .zip(["DSCF0001.RAF", "DSCF9999.RAF", "DSCF0001-1.RAF"])
    {
        assert_eq!(fs::read(library.join(target)).unwrap(), raf(&picture(name)));
    }
    assert_eq!(second.status.code(), Some(0), "{}", stderr(&second));
    for name in names {
        assert!(has_line_starting_with(
            &second,
            &format!("imported before  {name} -> ")
        ));
    }
    assert!(has_line(
        &second,
        "0 copied, 3 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out"
    ));
}
