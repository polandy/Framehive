// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Start, destination, waiting for cards, quitting and the remembered answers.

use std::path::Path;

use domain::Timestamp;

use super::support::{Harness, OCTOBER_17, PICTURES, PRESET_1, Step, mount};

const WAITING: &str = "Waiting for a memory card…\n";

/// A harness with a remembered destination and template.
fn remembered() -> Harness {
    let harness = Harness::new();
    harness.remember(Some(PICTURES), Some(PRESET_1), &[]);
    harness
}

/// spec: CLI-002.2
#[test]
fn the_first_run_asks_for_the_destination() {
    let harness = Harness::new();

    let transcript = harness.run(vec![Step::Line("")]);

    assert_eq!(
        transcript.output,
        format!(
            "Framehive guided import. Type q and Enter to quit.\n\
             Where should the photos go? [/home/ada/Pictures]: \n{WAITING}\n"
        )
    );
    assert!(
        harness
            .state_text()
            .contains("destination=/home/ada/Pictures\n")
    );
}

/// spec: CLI-002.3
#[test]
fn home_and_relative_destinations_are_resolved() {
    let harness = Harness::new();
    harness.run(vec![Step::Line("~/Fotos")]);
    assert!(
        harness
            .state_text()
            .contains("destination=/home/ada/Fotos\n")
    );

    let harness = Harness::new();
    harness.run(vec![Step::Line("cards")]);
    assert!(
        harness
            .state_text()
            .contains("destination=/tmp/work/cards\n")
    );
}

/// spec: CLI-002.4
#[test]
fn waiting_lists_cards_as_they_come_and_go() {
    let harness = remembered();
    harness
        .memory
        .add_folder(&mount("EOS_DIGITAL").join("DCIM"));
    harness.add_file(
        "EOS_DIGITAL",
        "DCIM/100_FUJI/DSCF0001.RAF",
        b"raf",
        OCTOBER_17,
    );

    let transcript = harness.run(vec![
        Step::Tick,
        Step::Insert("EOS_DIGITAL"),
        Step::Tick,
        Step::Tick,
        Step::Remove("EOS_DIGITAL"),
        Step::Tick,
    ]);

    let output = &transcript.output;
    let waiting = output.find(WAITING).unwrap();
    let new = output
        .find("[NEW] EOS_DIGITAL (/run/media/ada/EOS_DIGITAL)\n")
        .unwrap();
    let removed = output.find("[REMOVED] EOS_DIGITAL\n").unwrap();
    assert!(waiting < new && new < removed, "{output}");
    assert!(output[removed..].contains(WAITING), "{output}");
}

/// spec: CLI-002.5
#[test]
fn a_single_card_is_read_at_once() {
    let harness = remembered();
    harness.insert("EOS_DIGITAL");

    let transcript = harness.run(vec![]);

    assert!(
        transcript
            .output
            .contains("[NEW] EOS_DIGITAL (/run/media/ada/EOS_DIGITAL)\nReading EOS_DIGITAL…\n")
    );
}

/// spec: CLI-002.5
#[test]
fn several_cards_are_asked_for_by_mount_point() {
    let harness = remembered();
    harness.insert("Untitled");
    harness.insert("EOS_DIGITAL");

    let transcript = harness.run(vec![Step::Line("")]);

    assert!(
        transcript.output.contains(
            "Which card?\n\
         \x20 1  EOS_DIGITAL (/run/media/ada/EOS_DIGITAL)\n\
         \x20 2  Untitled (/run/media/ada/Untitled)\n\
         Choose 1–2 [1]: \n\
         Reading EOS_DIGITAL…\n"
        ),
        "{}",
        transcript.output
    );
}

/// spec: CLI-002.6
#[test]
fn a_card_with_nothing_to_import_is_not_offered_again_until_reinserted() {
    let harness = remembered();
    harness.insert("EOS_DIGITAL");

    let transcript = harness.run(vec![
        Step::Tick,
        Step::Tick,
        Step::Remove("EOS_DIGITAL"),
        Step::Tick,
        Step::Insert("EOS_DIGITAL"),
        Step::Tick,
    ]);

    let nothing = "Reading EOS_DIGITAL…\nEOS_DIGITAL has no photos or videos to import.\n";
    assert_eq!(
        transcript.output,
        format!(
            "Framehive guided import. Type q and Enter to quit.\n{WAITING}\
             [NEW] EOS_DIGITAL (/run/media/ada/EOS_DIGITAL)\n{nothing}{WAITING}\
             [REMOVED] EOS_DIGITAL\n\
             [NEW] EOS_DIGITAL (/run/media/ada/EOS_DIGITAL)\n{nothing}{WAITING}\n"
        )
    );
}

/// spec: CLI-002.14
#[test]
fn q_quits_at_any_prompt_and_copies_nothing() {
    let harness = Harness::new();
    let transcript = harness.run(vec![Step::Line("q")]);
    assert!(transcript.output.ends_with("[/home/ada/Pictures]: q\n"));

    let harness = remembered();
    harness.insert("EOS_DIGITAL");
    harness.add_file(
        "EOS_DIGITAL",
        "DCIM/100_FUJI/DSCF0001.RAF",
        b"raf",
        OCTOBER_17,
    );
    let transcript = harness.run(vec![Step::Line(" q ")]);
    assert!(
        transcript.output.ends_with("[q] quit:  q \n"),
        "{}",
        transcript.output
    );
    assert!(
        !harness
            .memory
            .paths()
            .iter()
            .any(|path| path.starts_with(PICTURES))
    );
}

/// spec: CLI-002.14
#[test]
fn the_end_of_input_quits() {
    let harness = remembered();
    harness.insert("EOS_DIGITAL");
    harness.add_file(
        "EOS_DIGITAL",
        "DCIM/100_FUJI/DSCF0001.RAF",
        b"raf",
        OCTOBER_17,
    );

    let transcript = harness.run(vec![Step::End]);

    assert!(transcript.output.ends_with("[q] quit: \n"));
    assert!(
        !harness
            .memory
            .paths()
            .iter()
            .any(|path| path.starts_with(PICTURES))
    );
}

/// spec: CLI-003.9
#[test]
fn remembered_answers_are_not_asked_again() {
    let harness = Harness::new();
    harness.insert("EOS_DIGITAL");
    harness.add_file(
        "EOS_DIGITAL",
        "DCIM/100_FUJI/DSCF0001.RAF",
        b"raf",
        OCTOBER_17,
    );
    harness.run(vec![
        Step::Line(""),
        Step::Line("2"),
        Step::Line("y"),
        Step::Line("Harbour"),
        Step::Line(""),
        Step::Line("n"),
    ]);

    let transcript = harness.run(vec![]);

    let output = &transcript.output;
    assert!(!output.contains("Where should the photos go?"), "{output}");
    assert!(
        !output.contains("How should the photos be named?"),
        "{output}"
    );
    assert!(
        output.contains("Job code:\n  1  Harbour\nChoose 1 or type a job code [1]: "),
        "{output}"
    );
}

/// spec: CLI-003.10
#[test]
fn a_broken_state_file_is_a_warning_and_the_first_run() {
    let harness = Harness::new();
    let modified = Timestamp::from_unix_seconds(OCTOBER_17);
    harness
        .memory
        .add_file(&Harness::state_file(), b"format_version=7\n", modified);

    let transcript = harness.run(vec![]);

    assert!(
        transcript.warnings.starts_with("warning: "),
        "{}",
        transcript.warnings
    );
    assert!(
        transcript
            .output
            .contains("Where should the photos go? [/home/ada/Pictures]: ")
    );
    assert!(Path::new(PICTURES).is_absolute());
}
