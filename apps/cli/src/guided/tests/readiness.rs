// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The format readiness at the end of the guided import (CLI-007).

use std::path::Path;
use std::time::Duration;

use domain::Timestamp;

use super::support::{Harness, OCTOBER_17, PICTURES, PRESET_1, Step};

const BACKUP: &str = "/mnt/backup";

/// A harness with the destination `PICTURES`, preset 1, the backup destination
/// `BACKUP` and the card `EOS_DIGITAL` with two Canon raw files from 17 October.
fn with_canon_card() -> Harness {
    let harness = Harness::new();
    harness.remember(Some(PICTURES), Some(PRESET_1), &[]);
    harness.remember_backups(&[BACKUP]);
    harness.insert("EOS_DIGITAL");
    for (path, content) in [
        ("DCIM/100CANON/IMG_0001.CR2", b"first"),
        ("DCIM/100CANON/IMG_0002.CR2", b"other"),
    ] {
        harness.add_file("EOS_DIGITAL", path, content, OCTOBER_17);
    }
    harness
}

/// spec: CLI-007.8
#[test]
fn the_guided_import_shows_the_format_readiness_before_the_eject_question() {
    let harness = with_canon_card();

    let transcript = harness.run(vec![Step::Line(""), Step::Line("n")]);

    let output = &transcript.output;
    assert!(
        output.contains(
            "\nbackup /mnt/backup: 2 backed up, 0 already backed up, 0 failed\n\
             The card can be formatted: every file is verified in 2 locations.\n\
             Eject EOS_DIGITAL? [Y/n]: n\n"
        ),
        "{output}"
    );
}

/// spec: CLI-007.9
#[test]
fn the_guided_import_lists_a_file_that_is_not_saved() {
    let harness = with_canon_card();
    harness.memory.add_file(
        &Path::new(BACKUP).join("2026/2026-10-17/IMG_0002.CR2"),
        b"occupied",
        Timestamp::from_unix_seconds(OCTOBER_17),
    );

    let transcript = harness.run(vec![Step::Line(""), Step::Line("n")]);

    let output = &transcript.output;
    assert!(
        output.contains(
            "\nbackup /mnt/backup: 1 backed up, 0 already backed up, 1 failed\n\
             Do not format the card: not every file is verified in 2 locations.\n\
             \x20 DCIM/100CANON/IMG_0002.CR2: verified in 1 of 2 locations\n\
             Eject EOS_DIGITAL? [Y/n]: n\n"
        ),
        "{output}"
    );
}

/// spec: CLI-007.10
#[test]
fn a_card_pulled_during_the_import_gets_one_format_readiness_for_the_whole_card() {
    let harness = Harness::new();
    harness.remember(Some(PICTURES), Some(PRESET_1), &[]);
    harness.insert("EOS_DIGITAL");
    for number in 1..=3u8 {
        let path = format!("DCIM/100_FUJI/DSCF000{number}.RAF");
        harness.add_file("EOS_DIGITAL", &path, &[number; 1000], OCTOBER_17);
    }
    harness.time_per_copy.set(Duration::from_secs(1));
    harness.pull_after_copies.set(Some(1));

    let transcript = harness.run(vec![
        Step::Line(""),
        Step::Tick,
        Step::Insert("EOS_DIGITAL"),
        Step::Tick,
        Step::Line("n"),
    ]);

    let output = &transcript.output;
    let readiness = "The card can be formatted: every file is verified in 1 location.\n";
    assert_eq!(output.matches(readiness).count(), 1, "{output}");
    assert!(!output.contains("Do not format"), "{output}");
    let resumed = output
        .find("EOS_DIGITAL was removed during the import. Insert it again to go on.\n")
        .unwrap_or_else(|| panic!("{output}"));
    assert!(
        output[resumed..].contains(&format!(
            "\n2 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n\
             {readiness}Eject EOS_DIGITAL? [Y/n]: n\n"
        )),
        "{output}"
    );
}
