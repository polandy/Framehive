// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The plan summary and its choices.

use std::path::Path;

use domain::Timestamp;

use super::support::{Harness, OCTOBER_17, OCTOBER_19, PICTURES, PRESET_1, PRESET_2, Step, mount};

const ON_CARD: &str = "The destination is on the card; type d to choose another.\n";

/// A harness with `destination`, `template` and the card `EOS_DIGITAL` with
/// `DSCF0001.RAF` from 17 October and `DSCF0002.RAF` from 19 October.
fn with_card(destination: &str, template: &str, job_codes: &[&str]) -> Harness {
    let harness = Harness::new();
    harness.remember(Some(destination), Some(template), job_codes);
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

/// spec: CLI-003.1
#[test]
fn the_summary_shows_files_size_dates_destination_template_and_job_code() {
    let harness = with_card(PICTURES, PRESET_2, &["Smith wedding"]);
    harness.pretend_size("EOS_DIGITAL", "DCIM/100_FUJI/DSCF0001.RAF", 1_500_000_000);
    harness.pretend_size("EOS_DIGITAL", "DCIM/100_FUJI/DSCF0002.RAF", 1_500_000_000);

    let transcript = harness.run(vec![Step::Line("")]);

    assert!(
        transcript.output.ends_with(
            "Choose 1 or type a job code [1]: \n\
         EOS_DIGITAL: 2 files, 3.0 GB, captured 2026-10-17 to 2026-10-19\n\
         \x20 to        /home/ada/Pictures\n\
         \x20 named     {date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}\n\
         \x20 job code  Smith wedding\n\
         [Enter] import  [l] list  [j] job code  [t] template  [d] destination  [q] quit: \n"
        ),
        "{}",
        transcript.output
    );
}

/// spec: CLI-003.2
#[test]
fn listing_prints_the_dry_run_lines_and_copies_nothing() {
    let harness = with_card(PICTURES, PRESET_1, &[]);
    let imported = Path::new(PICTURES).join("2026/2026-10-17/DSCF0001.RAF");
    harness.memory.add_file(
        &imported,
        b"first",
        Timestamp::from_unix_seconds(OCTOBER_17),
    );

    let transcript = harness.run(vec![Step::Line("l")]);

    let output = &transcript.output;
    let listed = output.find("[q] quit: l\n").unwrap();
    let listing = &output[listed..];
    assert!(
        listing.contains(
            "\nalready imported  DCIM/100_FUJI/DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF\n\
         would copy  DCIM/100_FUJI/DSCF0002.RAF -> 2026/2026-10-19/DSCF0002.RAF"
        ),
        "{output}"
    );
    assert!(
        listing.contains(
            "\n1 would copy, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n\
         EOS_DIGITAL: 2 files, "
        ),
        "{output}"
    );
    let not_copied = Path::new(PICTURES).join("2026/2026-10-19/DSCF0002.RAF");
    assert_eq!(harness.memory.content(&not_copied), None);
}

/// spec: CLI-003.3
#[test]
fn the_destination_can_be_changed_at_the_summary() {
    let harness = with_card(PICTURES, PRESET_1, &[]);

    let transcript = harness.run(vec![Step::Line("d"), Step::Line("/home/ada/Archive")]);

    assert!(
        transcript.output.contains(
            "[q] quit: d\nWhere should the photos go? [/home/ada/Pictures]: /home/ada/Archive\n\
         EOS_DIGITAL: 2 files, 0.0 MB, captured 2026-10-17 to 2026-10-19\n\
         \x20 to        /home/ada/Archive\n"
        ),
        "{}",
        transcript.output
    );
    assert!(
        harness
            .state_text()
            .contains("destination=/home/ada/Archive\n")
    );
}

/// spec: CLI-003.4
#[test]
fn a_destination_on_the_card_does_not_start() {
    let destination = mount("EOS_DIGITAL").join("import");
    let harness = with_card(destination.to_str().unwrap(), PRESET_1, &[]);

    let transcript = harness.run(vec![Step::Line("")]);

    assert!(transcript.output.contains(&format!(
        "\x20 named     {PRESET_1}\n{ON_CARD}[Enter] import  [l] list  [t] template  [d] destination  [q] quit: \n{ON_CARD}"
    )), "{}", transcript.output);
    assert!(
        !harness
            .memory
            .paths()
            .iter()
            .any(|path| path.starts_with(&destination))
    );
}
