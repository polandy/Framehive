// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The dialog of the guided import (CLI-002) against the binary on a pseudo-terminal,
//! with cards listed in a mount table file (ADR 0050).

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::process::Command;

use support::guided_card::{Machine, text};
use support::terminal::{Transcript, run_on_terminal, run_on_terminal_with};

const PRESET_1: &str = "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}";
const PRESET_2: &str = "{date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}";
const PRESET_3: &str = "{date:%Y}/{date:%Y-%m}/{date:%Y%m%d}_{number:4}.{ext}";
const PRESETS: &str = "How should the photos be named?\n\
    \x20 1  By day:              {date:%Y}/{date:%Y-%m-%d}/{name}.{ext}\n\
    \x20 2  By day and job code: {date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}\n\
    \x20 3  By month, renamed:   {date:%Y}/{date:%Y-%m}/{date:%Y%m%d}_{number:4}.{ext}\n\
    Choose 1–3 or type a template [1]: ";
const WAITING: &str = "Waiting for a memory card or camera…";
const NOTHING_TO_IMPORT: &str = "EOS_DIGITAL has no photos or videos to import.";
const NO_COMMAND: &str = "Insert a memory card or connect a camera, or type q to quit.";

fn successful(transcript: Transcript) -> Transcript {
    assert_eq!(transcript.status.code(), Some(0), "{}", transcript.errors);
    transcript
}

/// A machine without a card that remembers a library and `template`.
fn remembering(template: Option<&str>, job_codes: &[&str]) -> Machine {
    let machine = Machine::without_card();
    machine.remember(&text(&machine.library()), template, job_codes);
    machine
}

/// A machine that remembers a library and `template`, with the card `EOS_DIGITAL` and
/// `DSCF0001.RAF` on it.
fn with_picture(template: Option<&str>, job_codes: &[&str]) -> Machine {
    let machine = remembering(template, job_codes);
    machine.set_cards(&["EOS_DIGITAL"]);
    machine.add_raw("DSCF0001.RAF");
    machine
}

/// spec: CLI-002.2
#[test]
fn the_first_run_asks_for_the_destination() {
    let machine = Machine::without_card();
    let default = machine.folders.home().join("Pictures");

    let run = successful(machine.run(&[""]));

    assert!(
        run.output.contains(&format!(
            "Where should the photos go? [{}]: ",
            text(&default)
        )),
        "{}",
        run.output
    );
    assert!(
        machine
            .state_text()
            .contains(&format!("destination={}\n", text(&default))),
        "{}",
        machine.state_text()
    );
}

/// spec: CLI-002.3
#[test]
fn home_and_relative_destinations_are_resolved() {
    let home_machine = Machine::without_card();
    successful(home_machine.run(&["~/Fotos"]));
    let relative_machine = Machine::without_card();
    successful(relative_machine.run(&["cards"]));

    let home = home_machine.folders.home().join("Fotos");
    assert!(
        home_machine
            .state_text()
            .contains(&format!("destination={}\n", text(&home))),
        "{}",
        home_machine.state_text()
    );
    let relative = relative_machine.root().join("cards");
    assert!(
        relative_machine
            .state_text()
            .contains(&format!("destination={}\n", text(&relative))),
        "{}",
        relative_machine.state_text()
    );
}

/// spec: CLI-002.4
#[test]
fn waiting_lists_cards_as_they_come_and_go() {
    let machine = remembering(Some(PRESET_1), &[]);
    let card = text(&machine.card());

    let run = successful(run_on_terminal_with(machine.command(), |terminal| {
        terminal.wait_for(WAITING);
        machine.set_cards(&["EOS_DIGITAL"]);
        terminal.wait_for(&format!("[NEW] EOS_DIGITAL ({card})\n"));
        machine.set_cards(&[]);
        terminal.wait_for("[REMOVED] EOS_DIGITAL\n");
    }));

    assert!(run.output.contains(WAITING), "{}", run.output);
}

/// spec: CLI-002.5
#[test]
fn one_card_is_chosen() {
    let machine = remembering(Some(PRESET_1), &[]);
    machine.set_cards(&["EOS_DIGITAL"]);

    let run = successful(machine.run(&[]));

    assert!(
        run.output.contains("Reading EOS_DIGITAL…\n"),
        "{}",
        run.output
    );
    assert!(
        !run.output.contains("Which card or camera?"),
        "{}",
        run.output
    );
}

/// spec: CLI-002.5
#[test]
fn several_cards_are_asked_for_and_enter_chooses_the_first() {
    let machine = remembering(Some(PRESET_1), &[]);
    machine.set_cards(&["EOS_DIGITAL", "Untitled"]);
    let media = text(&machine.media());

    let run = successful(machine.run(&[""]));

    assert!(
        run.output.contains(&format!(
            "Which card or camera?\n  1  EOS_DIGITAL ({media}/EOS_DIGITAL)\n  2  Untitled ({media}/Untitled)\n\
             Choose 1–2 [1]: Reading EOS_DIGITAL…\n"
        )),
        "{}",
        run.output
    );
}

/// spec: CLI-002.6
#[test]
fn a_card_with_nothing_to_import_is_not_offered_again_until_it_is_inserted_again() {
    let machine = remembering(Some(PRESET_1), &[]);
    machine.set_cards(&["EOS_DIGITAL"]);

    let run = successful(run_on_terminal_with(machine.command(), |terminal| {
        terminal.wait_for(NOTHING_TO_IMPORT);
        // Each line makes the dialog poll once more, with the card still present.
        for _ in 0..2 {
            terminal.type_line("x");
            terminal.wait_for(NO_COMMAND);
        }
        machine.set_cards(&[]);
        terminal.wait_for("[REMOVED] EOS_DIGITAL\n");
        machine.set_cards(&["EOS_DIGITAL"]);
        terminal.wait_for(NOTHING_TO_IMPORT);
    }));

    assert_eq!(run.output.matches("Reading EOS_DIGITAL…").count(), 2);
    assert_eq!(run.output.matches(NOTHING_TO_IMPORT).count(), 2);
}

/// spec: CLI-002.7
#[test]
fn a_preset_is_previewed_and_remembered() {
    let machine = remembering(None, &[]);
    machine.set_cards(&["EOS_DIGITAL"]);
    machine.add_raw("DSCF0001.RAF");

    let run = successful(machine.run(&["3", "y", "q"]));

    assert!(
        run.output.contains(
            "  DCIM/100_FUJI/DSCF0001.RAF -> 2026/2026-10/20261017_0001.RAF\n\
             Use this template? [Y/n]: "
        ),
        "{}",
        run.output
    );
    assert!(
        machine
            .state_text()
            .contains(&format!("template={PRESET_3}\n")),
        "{}",
        machine.state_text()
    );
}

/// spec: CLI-002.8
#[test]
fn an_invalid_template_is_shown_with_a_caret() {
    let machine = with_picture(None, &[]);

    let run = successful(machine.run(&["{date:%Y}/{colour}/{name}.{ext}"]));

    let error = "  {date:%Y}/{colour}/{name}.{ext}\n            ^ ";
    let shown = run.output.find(error).expect(&run.output);
    assert!(run.output[shown..].contains(PRESETS), "{}", run.output);
}

/// spec: CLI-002.9
#[test]
fn a_rejected_preview_starts_the_step_again() {
    let machine = with_picture(None, &[]);

    let run = successful(machine.run(&["1", "n"]));

    assert!(
        run.output
            .contains(&format!("Use this template? [Y/n]: {PRESETS}")),
        "{}",
        run.output
    );
    assert!(!machine.state_text().contains("template="));
}

/// spec: CLI-002.10
#[test]
fn the_preview_shows_the_first_five_files() {
    let machine = with_picture(None, &[]);
    machine.add_raw("ABC.RAF");
    for number in 2..=6 {
        machine.add_raw(&format!("DSCF000{number}.RAF"));
    }

    let run = successful(machine.run(&["3", "n"]));

    assert!(
        run.output.contains(
            "Preview with the first 5 files of EOS_DIGITAL:\n\
             \x20 DCIM/100_FUJI/ABC.RAF -> cannot be named: {number:4} needs an image number, but \"ABC\" does not end in a digit\n\
             \x20 DCIM/100_FUJI/DSCF0001.RAF -> 2026/2026-10/20261017_0001.RAF\n\
             \x20 DCIM/100_FUJI/DSCF0002.RAF -> 2026/2026-10/20261017_0002.RAF\n\
             \x20 DCIM/100_FUJI/DSCF0003.RAF -> 2026/2026-10/20261017_0003.RAF\n\
             \x20 DCIM/100_FUJI/DSCF0004.RAF -> 2026/2026-10/20261017_0004.RAF\n\
             Use this template?"
        ),
        "{}",
        run.output
    );
}

/// spec: CLI-002.11
#[test]
fn a_template_without_job_asks_for_no_job_code() {
    let machine = with_picture(Some(PRESET_1), &["Harbour"]);

    let run = successful(machine.run(&["q"]));

    assert!(!run.output.contains("Job code"), "{}", run.output);
    assert!(!run.output.contains("job code"), "{}", run.output);
    assert!(!run.output.contains("[j]"), "{}", run.output);
    assert!(run.output.contains("[Enter] import"), "{}", run.output);
}

/// spec: CLI-002.12
#[test]
fn a_remembered_job_code_is_chosen_by_number() {
    let machine = with_picture(Some(PRESET_2), &["Smith wedding", "Harbour"]);

    let run = successful(machine.run(&["2", "", "n"]));

    assert!(
        run.output.contains(
            "Job code:\n  1  Smith wedding\n  2  Harbour\nChoose 1–2 or type a job code [1]: "
        ),
        "{}",
        run.output
    );
    let copied = machine
        .library()
        .join("2026/2026-10-17_Harbour/DSCF0001.RAF");
    assert!(copied.is_file(), "{}", run.output);
    assert!(
        machine
            .state_text()
            .ends_with("job_code=Harbour\njob_code=Smith wedding\n"),
        "{}",
        machine.state_text()
    );
}

/// spec: CLI-002.13
#[test]
fn a_new_job_code_is_checked() {
    let machine = with_picture(Some(PRESET_2), &[]);

    let run = successful(machine.run(&["", "Smith/Jones", "Smith wedding", "q"]));

    assert!(
        run.output.contains(
            "Job code: A job code is needed, because the template has {job}.\n\
             Job code: Invalid job code: a job code must not contain \"/\".\n\
             Job code: "
        ),
        "{}",
        run.output
    );
    assert!(
        run.output.contains("  job code  Smith wedding\n"),
        "{}",
        run.output
    );
}

/// spec: CLI-002.14
#[test]
fn q_quits_without_copying() {
    let machine = with_picture(Some(PRESET_1), &[]);

    let run = successful(machine.run(&["q"]));

    assert!(run.output.contains("[Enter] import"), "{}", run.output);
    assert!(!machine.library().exists());
}

/// spec: CLI-002.14
#[test]
fn the_end_of_input_quits_without_copying() {
    let machine = with_picture(Some(PRESET_1), &[]);

    let run = successful(machine.run(&[]));

    assert!(run.output.contains("[Enter] import"), "{}", run.output);
    assert!(!machine.library().exists());
}

/// spec: CLI-002.15
#[test]
fn a_relative_home_folder_is_unknown() {
    let machine = Machine::without_card();
    let mut command = machine.command();
    command.env("HOME", "ada");

    let run = run_on_terminal(command, &[]);

    assert_eq!(run.errors, "error: the home folder is unknown; set HOME\n");
    assert_eq!(run.status.code(), Some(2));
}

/// spec: CLI-002.15
#[test]
fn an_unreadable_current_folder_is_unknown() {
    let machine = Machine::without_card();
    fs::create_dir(machine.root().join("gone")).unwrap();
    // The shell deletes its current folder, then replaces itself with the binary.
    let mut command = Command::new("sh");
    command
        .args([
            "-c",
            "cd gone && rmdir ../gone && exec \"$0\"",
            env!("CARGO_BIN_EXE_framehive"),
        ])
        .env("TZ", "UTC")
        .env("HOME", machine.folders.home())
        .env("XDG_STATE_HOME", machine.root().join("state"))
        .current_dir(machine.root());

    let run = run_on_terminal(command, &[]);

    assert!(
        run.errors
            .starts_with("error: the current folder cannot be read: "),
        "{}",
        run.errors
    );
    assert_eq!(run.status.code(), Some(2));
}
