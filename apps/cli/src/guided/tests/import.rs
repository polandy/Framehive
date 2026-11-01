// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import, its progress and report, the eject and a card pulled meanwhile.

use std::path::{Path, PathBuf};
use std::time::Duration;

use devices::DeviceSource;

use super::support::{Harness, OCTOBER_17, PICTURES, PRESET_1, Step};

const MEGABYTES_10: usize = 10_000_000;

/// A harness with a remembered destination and preset 1, the clock advancing one
/// second per copy, and the card `EOS_DIGITAL` with three files of 10 MB.
fn with_three_files() -> Harness {
    let harness = Harness::new();
    harness.remember(Some(PICTURES), Some(PRESET_1), &[]);
    harness.insert("EOS_DIGITAL");
    for number in 1..=3u8 {
        let path = format!("DCIM/100_FUJI/DSCF000{number}.RAF");
        harness.add_file(
            "EOS_DIGITAL",
            &path,
            &vec![number; MEGABYTES_10],
            OCTOBER_17,
        );
    }
    harness.time_per_copy.set(Duration::from_secs(1));
    harness
}

fn imported(number: u8) -> PathBuf {
    Path::new(PICTURES).join(format!("2026/2026-10-17/DSCF000{number}.RAF"))
}

/// The states of the progress line in `output`, without the blanks of a redraw.
fn progress_states(output: &str) -> Vec<&str> {
    let start = output.find('\r').unwrap();
    let end = output[start..].find('\n').unwrap() + start;
    output[start..end]
        .split('\r')
        .skip(1)
        .map(str::trim_end)
        .collect()
}

/// spec: CLI-003.5
#[test]
fn the_import_redraws_the_progress_and_reports() {
    let harness = with_three_files();

    let transcript = harness.run(vec![Step::Line(""), Step::Line("n")]);

    let output = &transcript.output;
    assert_eq!(
        progress_states(output),
        [
            "  0/3 files  0.0/30.0 MB  -- MB/s  --:-- left",
            "  1/3 files  10.0/30.0 MB  10 MB/s  0:02 left",
            "  2/3 files  20.0/30.0 MB  10 MB/s  0:01 left",
            "  3/3 files  30.0/30.0 MB  10 MB/s  0:00 left",
        ]
    );
    assert!(
        output.contains(
            "  3/3 files  30.0/30.0 MB  10 MB/s  0:00 left\n\
         3 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n\
         Eject EOS_DIGITAL? [Y/n]: n\n"
        ),
        "{output}"
    );
    for number in 1..=3 {
        assert_eq!(
            harness.memory.content(&imported(number)),
            Some(vec![number; MEGABYTES_10])
        );
    }
}

/// spec: CLI-003.6
#[test]
fn enter_ejects_the_card() {
    let harness = with_three_files();

    let transcript = harness.run(vec![Step::Line(""), Step::Line(""), Step::Tick]);

    let output = &transcript.output;
    assert!(output.ends_with(
        "Eject EOS_DIGITAL? [Y/n]: \nEOS_DIGITAL can be removed now.\nWaiting for a memory card…\n\n"
    ), "{output}");
    assert!(harness.devices.list().unwrap().is_empty());
}

/// spec: CLI-003.7
#[test]
fn a_failed_eject_is_reported() {
    let harness = with_three_files();
    harness.devices.fail_ejects(Some("target is busy"));

    let transcript = harness.run(vec![Step::Line(""), Step::Line("y")]);

    assert!(
        transcript.output.ends_with(
            "Eject EOS_DIGITAL? [Y/n]: y\n\
         Could not eject EOS_DIGITAL: target is busy\n\
         Waiting for a memory card…\n\n"
        ),
        "{}",
        transcript.output
    );
}

/// spec: CLI-003.8
#[test]
fn a_card_pulled_during_the_import_is_imported_again_when_it_is_back() {
    let harness = with_three_files();
    harness.pull_after_copies.set(Some(1));

    let transcript = harness.run(vec![
        Step::Line(""),
        Step::Tick,
        Step::Insert("EOS_DIGITAL"),
        Step::Tick,
        Step::Line("n"),
    ]);

    let output = &transcript.output;
    let stopped = output
        .find(
            "\n[REMOVED] EOS_DIGITAL\n\
             EOS_DIGITAL was removed during the import. Insert it again to go on.\n\
             [NEW] EOS_DIGITAL (/run/media/ada/EOS_DIGITAL)\n\
             Reading EOS_DIGITAL…\n",
        )
        .unwrap_or_else(|| panic!("{output}"));
    assert!(
        output[..stopped].ends_with("  2/3 files  20.0/30.0 MB  10 MB/s  0:01 left"),
        "{output}"
    );
    assert!(
        output[stopped..].contains(
            "\n2 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n\
         Eject EOS_DIGITAL? [Y/n]: n\n"
        ),
        "{output}"
    );
    for number in 1..=3 {
        assert_eq!(
            harness.memory.content(&imported(number)),
            Some(vec![number; MEGABYTES_10])
        );
    }
}
