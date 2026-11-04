// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Backup destinations in the guided import (CLI-006).

use std::path::{Path, PathBuf};

use domain::Timestamp;

use super::support::{Harness, OCTOBER_17, OCTOBER_19, PICTURES, PRESET_1, Step, mount};
use crate::guided::state::parse_state;

const PROMPT: &str =
    "[Enter] import  [l] list  [t] template  [d] destination  [b] backup  [q] quit: ";
const BACKUP_PROMPT: &str =
    "Backup folder to add, or the number of one to remove [Enter keeps them]: ";

/// A harness with the destination `PICTURES`, preset 1, the backup destinations
/// `backups` and the card `EOS_DIGITAL` with `DSCF0001.RAF` from 17 October and
/// `DSCF0002.RAF` from 19 October.
fn with_card(backups: &[&str]) -> Harness {
    let harness = Harness::new();
    harness.remember(Some(PICTURES), Some(PRESET_1), &[]);
    harness.remember_backups(backups);
    harness.insert("EOS_DIGITAL");
    harness.add_file(
        "EOS_DIGITAL",
        "DCIM/100_FUJI/DSCF0001.RAF",
        b"first",
        OCTOBER_17,
    );
    harness.add_file(
        "EOS_DIGITAL",
        "DCIM/100_FUJI/DSCF0002.RAF",
        b"second",
        OCTOBER_19,
    );
    harness
}

fn summary_lines(backups: &str) -> String {
    format!("\x20 to        {PICTURES}\n{backups}\x20 named     {PRESET_1}\n{PROMPT}")
}

/// spec: CLI-006.1
#[test]
fn a_backup_destination_is_added_at_the_summary() {
    let harness = with_card(&[]);

    let transcript = harness.run(vec![
        Step::Line("b"),
        Step::Line("/run/media/ada/Backup/Pictures"),
    ]);

    let output = &transcript.output;
    let added = format!(
        "{}b\n{BACKUP_PROMPT}/run/media/ada/Backup/Pictures\n\
         EOS_DIGITAL: 2 files, 0.0 MB, captured 2026-10-17 to 2026-10-19\n{}",
        summary_lines("\x20 backup    none\n"),
        summary_lines("\x20 backup    /run/media/ada/Backup/Pictures\n")
    );
    assert!(output.contains(&added), "{output}");
    assert!(
        harness
            .state_text()
            .contains("\nbackup=/run/media/ada/Backup/Pictures\n"),
        "{}",
        harness.state_text()
    );
}

/// spec: CLI-006.2
#[test]
fn a_backup_destination_is_removed_by_its_number() {
    let harness = with_card(&["/mnt/a", "/mnt/b"]);

    let transcript = harness.run(vec![Step::Line("b"), Step::Line("1")]);

    let output = &transcript.output;
    let removed = format!(
        "{PROMPT}b\n  1  /mnt/a\n  2  /mnt/b\n{BACKUP_PROMPT}1\n\
         EOS_DIGITAL: 2 files, 0.0 MB, captured 2026-10-17 to 2026-10-19\n{}",
        summary_lines("\x20 backup    /mnt/b\n")
    );
    assert!(output.contains(&removed), "{output}");
    let state = harness.state_text();
    assert!(state.contains("\nbackup=/mnt/b\n"), "{state}");
    assert!(!state.contains("/mnt/a"), "{state}");
}

/// spec: CLI-006.2
#[test]
fn the_summary_lists_every_backup_destination_in_order() {
    let harness = with_card(&["/mnt/a", "/mnt/b"]);

    let transcript = harness.run(vec![]);

    let output = &transcript.output;
    let listed = summary_lines("\x20 backup    /mnt/a\n\x20 backup    /mnt/b\n");
    assert!(output.contains(&listed), "{output}");
}

/// spec: CLI-006.3
#[test]
fn a_backup_destination_on_the_card_does_not_start() {
    let backup = mount("EOS_DIGITAL").join("backup");
    let harness = with_card(&[backup.to_str().unwrap()]);

    let transcript = harness.run(vec![Step::Line("")]);

    let refused = format!(
        "The backup folder {} cannot be used: it is on the card; type b to change it.\n",
        backup.display()
    );
    let output = &transcript.output;
    assert!(
        output.contains(&format!(
            "\x20 named     {PRESET_1}\n{refused}{PROMPT}\n{refused}"
        )),
        "{output}"
    );
    let written = harness.memory.paths();
    assert!(
        !written
            .iter()
            .any(|path| path.starts_with(&backup) || path.starts_with(PICTURES)),
        "{written:?}"
    );
}

#[test]
fn a_backup_destination_inside_the_destination_does_not_start() {
    let backup = Path::new(PICTURES).join("backup");
    let harness = with_card(&[backup.to_str().unwrap()]);

    let transcript = harness.run(vec![Step::Line("")]);

    let output = &transcript.output;
    assert!(
        output.contains(&format!(
            "The backup folder {} cannot be used: it lies inside the import destination; type b to change it.\n{PROMPT}\n",
            backup.display()
        )),
        "{output}"
    );
    assert_eq!(
        harness
            .memory
            .content(&Path::new(PICTURES).join("2026/2026-10-17/DSCF0001.RAF")),
        None
    );
}

fn imported(folder: &str, name: &str) -> PathBuf {
    Path::new(folder).join(name)
}

const FIRST: &str = "2026/2026-10-17/DSCF0001.RAF";
const SECOND: &str = "2026/2026-10-19/DSCF0002.RAF";

/// spec: CLI-006.4
#[test]
fn the_import_writes_to_the_backup_destination() {
    let harness = with_card(&["/mnt/backup"]);

    let transcript = harness.run(vec![Step::Line(""), Step::Line("n")]);

    for folder in [PICTURES, "/mnt/backup"] {
        assert_eq!(
            harness.memory.content(&imported(folder, FIRST)),
            Some(b"first".to_vec())
        );
        assert_eq!(
            harness.memory.content(&imported(folder, SECOND)),
            Some(b"second".to_vec())
        );
    }
    let output = &transcript.output;
    assert!(
        output.contains(
            "\n2 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n\
             backup /mnt/backup: 2 backed up, 0 already backed up, 0 failed\n\
             Eject EOS_DIGITAL? [Y/n]: n\n"
        ),
        "{output}"
    );
}

/// spec: CLI-006.5
#[test]
fn a_failed_backup_is_reported() {
    let harness = with_card(&["/mnt/backup"]);
    let occupied = imported("/mnt/backup", FIRST);
    harness.memory.add_file(
        &occupied,
        b"other content",
        Timestamp::from_unix_seconds(OCTOBER_17),
    );

    let transcript = harness.run(vec![Step::Line(""), Step::Line("n")]);

    let output = &transcript.output;
    let copied = output
        .find("\ncopied  DCIM/100_FUJI/DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF")
        .unwrap_or_else(|| panic!("{output}"));
    let report = &output[copied + 1..];
    let mut lines = report.lines();
    lines.next();
    assert_eq!(
        lines.next(),
        Some("  backup failed  /mnt/backup: name conflict")
    );
    assert!(
        report.contains("\nbackup /mnt/backup: 1 backed up, 0 already backed up, 1 failed\n"),
        "{output}"
    );
    assert_eq!(
        harness.memory.content(&occupied),
        Some(b"other content".to_vec())
    );
}

/// spec: CLI-006.6
#[test]
fn backup_destinations_are_remembered() {
    let harness = with_card(&[]);
    harness.run(vec![Step::Line("b"), Step::Line("/mnt/backup")]);

    let transcript = harness.run(vec![]);

    let output = &transcript.output;
    assert!(
        output.ends_with(&format!(
            "{}\n",
            summary_lines("\x20 backup    /mnt/backup\n")
        )),
        "{output}"
    );
    assert!(!output.contains(BACKUP_PROMPT), "{output}");
}

/// spec: CLI-006.7
#[test]
fn a_state_file_of_version_1_is_read_and_written_as_version_2() {
    let harness = with_card(&[]);
    harness.memory.add_file(
        &Harness::state_file(),
        format!("format_version=1\ndestination={PICTURES}\ntemplate={PRESET_1}\n").as_bytes(),
        Timestamp::from_unix_seconds(OCTOBER_19),
    );

    let transcript = harness.run(vec![Step::Line("d"), Step::Line("")]);

    assert_eq!(transcript.warnings, "");
    let output = &transcript.output;
    assert!(
        output.contains(&summary_lines("\x20 backup    none\n")),
        "{output}"
    );
    assert_eq!(
        output.matches("Where should the photos go?").count(),
        1,
        "{output}"
    );
    assert!(
        harness.state_text().contains("\nformat_version=2\n"),
        "{}",
        harness.state_text()
    );
}

/// spec: CLI-006.7
#[test]
fn a_version_1_file_has_no_backup_destinations() {
    let loaded = parse_state("format_version=1\ndestination=/home/ada/Pictures\n").unwrap();

    assert!(loaded.warnings.is_empty());
    assert_eq!(
        loaded.state.destination.as_deref(),
        Some(Path::new("/home/ada/Pictures"))
    );
    assert!(loaded.state.backups.is_empty());
}

/// spec: CLI-006.1
#[test]
fn backup_lines_are_written_after_the_destination_and_read_back_in_order() {
    let text = "format_version=2\ndestination=/home/ada/Pictures\n\
                backup=/mnt/b\nbackup=/mnt/a\nbackup=relative\n";

    let loaded = parse_state(text).unwrap();

    assert_eq!(
        loaded.state.backups,
        [PathBuf::from("/mnt/b"), PathBuf::from("/mnt/a")]
    );
    assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
    assert!(
        crate::guided::state::format_state(&loaded.state)
            .contains("\ndestination=/home/ada/Pictures\nbackup=/mnt/b\nbackup=/mnt/a\n")
    );
}
