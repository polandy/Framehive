// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The backup destinations at the plan summary of the guided import (CLI-006) against
//! the binary on a pseudo-terminal, with a card listed in a mount table file (ADR 0050).

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use support::guided_card::{Machine, text};

/// A machine with the card `EOS_DIGITAL`, one picture and a remembered library and
/// `backups`.
fn machine_with_backups(backups: &[&str]) -> Machine {
    let machine = Machine::new();
    machine.remember_backups(&text(&machine.library()), backups);
    machine.add_pictures(&["DSCF0001.JPG"]);
    machine
}

/// spec: CLI-006.1
#[test]
fn a_backup_destination_is_added_at_the_summary_and_remembered() {
    let machine = machine_with_backups(&[]);
    let backup = text(&machine.backup());

    let run = machine.run(&["b", &backup]);

    assert!(
        run.output
            .contains("Backup folder to add, or the number of one to remove [Enter keeps them]: "),
        "{}",
        run.output
    );
    assert!(
        run.output.contains(&format!("\x20 backup    {backup}\n")),
        "{}",
        run.output
    );
    assert!(
        machine
            .state_text()
            .lines()
            .any(|line| line == format!("backup={backup}")),
        "{}",
        machine.state_text()
    );
}

/// spec: CLI-006.2
#[test]
fn a_backup_destination_is_removed_by_its_number_and_remembered() {
    let machine = machine_with_backups(&["/mnt/a", "/mnt/b"]);

    let run = machine.run(&["b", "1"]);

    let after = &run.output[run.output.find("Backup folder to add").expect(&run.output)..];
    assert!(after.contains("\x20 backup    /mnt/b\n"), "{}", run.output);
    assert!(!after.contains("/mnt/a\n"), "{}", run.output);
    let state = machine.state_text();
    assert!(state.contains("backup=/mnt/b\n"), "{state}");
    assert!(!state.contains("backup=/mnt/a"), "{state}");
}

/// spec: CLI-006.3
#[test]
fn a_backup_destination_on_the_card_is_refused_and_nothing_is_copied() {
    let machine = Machine::new();
    let backup = text(&machine.card().join("backup"));
    machine.remember_backups(&text(&machine.library()), &[&backup]);
    machine.add_pictures(&["DSCF0001.JPG"]);

    let run = machine.run(&[""]);

    assert!(
        run.output
            .contains(&format!("The backup folder {backup} cannot be used: ")),
        "{}",
        run.output
    );
    assert!(
        run.output.contains("; type b to change it."),
        "{}",
        run.output
    );
    assert!(!machine.library().exists());
    assert!(!machine.card().join("backup").exists());
}
