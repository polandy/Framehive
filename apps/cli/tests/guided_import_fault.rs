// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A guided import whose job fails in the import itself (CLI-003) against the binary
//! built with the fault-injection point (ADR 0048), on a pseudo-terminal.

#![cfg(all(target_os = "linux", feature = "fault-injection"))]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use support::guided_card::{Machine, text};
use support::terminal::{run_on_terminal, run_on_terminal_with};

const PRESET_1: &str = "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}";

/// spec: CLI-003.15
#[test]
fn an_import_that_stops_unexpectedly_shows_the_summary_again() {
    let machine = Machine::new();
    machine.remember(&text(&machine.library()), Some(PRESET_1), &[]);
    machine.add_raw("DSCF0001.RAF");
    let mut command = machine.command();
    command.env("FRAMEHIVE_FAULT", "panic-while-writing DSCF0001.RAF");

    let run = run_on_terminal(command, &["", "q"]);

    assert_eq!(run.status.code(), Some(0), "{}", run.errors);
    let output = &run.output;
    let stopped = output
        .find("Could not import EOS_DIGITAL: the import stopped unexpectedly\n")
        .unwrap_or_else(|| panic!("{output}"));
    assert!(
        output[stopped..].contains(&format!("\x20 named     {PRESET_1}\n")),
        "{output}"
    );
    assert!(!output.contains("copied  "), "{output}");
    let copied = fs::read_dir(machine.library()).map_or(0, |entries| {
        entries
            .flatten()
            .filter(|entry| entry.path().is_file())
            .count()
    });
    assert_eq!(copied, 0);
    assert!(
        !machine
            .library()
            .join("2026/2026-10-17/DSCF0001.RAF")
            .exists()
    );
}

/// spec: CLI-007.10
#[test]
fn a_card_pulled_during_the_import_gets_one_format_readiness_for_the_whole_card() {
    let machine = Machine::new();
    machine.remember(&text(&machine.library()), Some(PRESET_1), &[]);
    machine.add_pictures(&["DSCF0001.RAF", "DSCF0002.RAF", "DSCF0003.RAF"]);
    let mut command = machine.command();
    command.env("FRAMEHIVE_FAULT", "pull-card-after DSCF0001.RAF");

    let run = run_on_terminal_with(command, |terminal| {
        terminal.type_line("");
        terminal.wait_for("Insert it again to go on.\n");
        machine.set_cards(&["EOS_DIGITAL"]);
        terminal.wait_for("Eject EOS_DIGITAL? [Y/n]: ");
        terminal.type_line("n");
    });

    assert_eq!(run.status.code(), Some(0), "{}", run.errors);
    let output = &run.output;
    let readiness = "The card can be formatted: every file is verified in 1 location.\n";
    assert_eq!(output.matches(readiness).count(), 1, "{output}");
    assert!(!output.contains("Do not format"), "{output}");
    let pulled = output
        .find("EOS_DIGITAL was removed during the import. Insert it again to go on.\n")
        .unwrap_or_else(|| panic!("{output}"));
    assert!(output[..pulled].find("The card can be formatted").is_none());
    assert!(
        output[pulled..].contains(&format!(
            "\n2 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n\
             {readiness}Eject EOS_DIGITAL? [Y/n]: "
        )),
        "{output}"
    );
}

/// spec: CLI-003.8
#[test]
fn a_card_pulled_during_the_import_is_imported_on_after_it_is_inserted_again() {
    let machine = Machine::new();
    machine.remember(&text(&machine.library()), Some(PRESET_1), &[]);
    machine.add_pictures(&["DSCF0001.RAF", "DSCF0002.RAF", "DSCF0003.RAF"]);
    let mut command = machine.command();
    command.env("FRAMEHIVE_FAULT", "pull-card-after DSCF0001.RAF");

    let run = run_on_terminal_with(command, |terminal| {
        terminal.type_line("");
        terminal.wait_for("Insert it again to go on.\n");
        assert!(
            !machine
                .library()
                .join("2026/2026-10-17/DSCF0003.RAF")
                .exists(),
            "the import went on after the card was pulled"
        );
        machine.set_cards(&["EOS_DIGITAL"]);
        terminal.wait_for("Eject EOS_DIGITAL? [Y/n]: ");
        terminal.type_line("n");
    });

    assert_eq!(run.status.code(), Some(0), "{}", run.errors);
    let output = &run.output;
    assert!(
        output.contains("EOS_DIGITAL was removed during the import. Insert it again to go on.\n"),
        "{output}"
    );
    assert!(
        output.contains(
            "2 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n"
        ),
        "{output}"
    );
    for name in ["DSCF0001.RAF", "DSCF0002.RAF", "DSCF0003.RAF"] {
        assert!(
            machine
                .library()
                .join("2026/2026-10-17")
                .join(name)
                .exists(),
            "{name}"
        );
    }
}
