// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import history in the guided import (CLI-008) against the binary built with the
//! fault-injection point (ADR 0048) and on a pseudo-terminal, with a card listed in a
//! mount table file (ADR 0050).

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use support::guided_card::{Machine, text};
use support::terminal::run_on_terminal;

const PRESET_1: &str = "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}";

/// A machine with the card `EOS_DIGITAL` and `DSCF0001.RAF`, whose remembered
/// destination lies below a file, so it cannot be resolved.
fn with_unresolvable_destination(template: Option<&str>) -> (Machine, String) {
    let machine = Machine::new();
    machine.add_raw("DSCF0001.RAF");
    let file = machine.root().join("not-a-folder");
    fs::write(&file, "").unwrap();
    let destination = text(&file.join("library"));
    machine.remember(&destination, template, &[]);
    (machine, destination)
}

fn files_below(folder: &std::path::Path) -> usize {
    fs::read_dir(folder).map_or(0, Iterator::count)
}

/// spec: CLI-008.6
#[cfg(feature = "fault-injection")]
#[test]
fn a_pulled_card_is_resumed_from_the_history() {
    use support::terminal::run_on_terminal_with;

    let machine = Machine::new();
    machine.remember(&text(&machine.library()), Some("{seq_stored:4}.{ext}"), &[]);
    machine.add_pictures(&["DSCF0001.RAF", "DSCF0002.RAF", "DSCF0003.RAF"]);
    let mut command = machine.command();
    command.env("FRAMEHIVE_FAULT", "pull-card-after 0001.RAF");

    let run = run_on_terminal_with(command, |terminal| {
        terminal.type_line("");
        terminal.wait_for("Insert it again to go on.\n");
        machine.set_cards(&["EOS_DIGITAL"]);
        terminal.wait_for("Eject EOS_DIGITAL? [Y/n]: ");
        terminal.type_line("n");
    });

    assert_eq!(run.status.code(), Some(0), "{}", run.errors);
    let output = &run.output;
    let pulled = output
        .find("EOS_DIGITAL was removed during the import. Insert it again to go on.\n")
        .unwrap_or_else(|| panic!("{output}"));
    let resumed = &output[pulled..];
    assert!(
        resumed.contains(
            "2 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n"
        ),
        "{output}"
    );
    assert!(!output.contains("not recognized"), "{output}");
    assert!(machine.library().join("0001.RAF").is_file());
    assert_eq!(files_below(&machine.library()), 3);
}

/// spec: CLI-008.7
#[test]
fn a_history_that_cannot_be_used_stops_the_import_of_the_card() {
    let machine = Machine::new();
    machine.remember(&text(&machine.library()), Some(PRESET_1), &[]);
    machine.add_raw("DSCF0001.RAF");
    let history = machine.folders.history_file();
    fs::create_dir_all(history.parent().unwrap()).unwrap();
    fs::write(&history, "not a database, not a database").unwrap();

    let run = machine.run(&["", "q"]);

    let message = format!("The import history {} cannot be used: ", text(&history));
    assert!(run.output.contains(&message), "{}", run.output);
    assert_eq!(files_below(&machine.library()), 0);
}

/// spec: CLI-008.8
#[test]
fn a_folder_that_cannot_be_resolved_stops_the_import_of_the_card() {
    let (machine, destination) = with_unresolvable_destination(Some(PRESET_1));

    let run = machine.run(&["", "q"]);

    let message = format!("Could not use {destination}: ");
    assert!(run.output.contains(&message), "{}", run.output);
    assert!(!machine.library().exists());
    assert!(!run.output.contains("copied  "), "{}", run.output);
}

/// spec: CLI-008.9
#[test]
fn a_template_is_not_previewed_when_the_folder_cannot_be_resolved() {
    let (machine, _) = with_unresolvable_destination(None);

    let run = run_on_terminal(machine.command(), &["1", "n", "q"]);

    let output = &run.output;
    let message = output
        .find("The template cannot be previewed: ")
        .unwrap_or_else(|| panic!("{output}"));
    let line_end = output[message..].find('\n').unwrap();
    assert!(
        output[message + line_end..].starts_with("\nUse this template? [Y/n]: "),
        "{output}"
    );
}
