// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The format readiness at the end of the guided import (CLI-007) against the binary on
//! a pseudo-terminal, with a card listed in a mount table file (ADR 0050).

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use support::guided_card::{Machine, text};

/// A machine with the card `EOS_DIGITAL`, two pictures, a remembered library and one
/// backup destination.
fn machine_with_backup() -> Machine {
    let machine = Machine::new();
    machine.add_pictures(&["DSCF0001.JPG", "DSCF0002.JPG"]);
    machine.remember_backups(&text(&machine.library()), &[&text(&machine.backup())]);
    machine
}

/// spec: CLI-007.8
#[test]
fn the_format_readiness_comes_before_the_eject_question() {
    let machine = machine_with_backup();

    let run = machine.run(&["", "n"]);

    assert!(
        run.output.contains(
            "The card can be formatted: every file is verified in 2 locations.\n\
             Eject EOS_DIGITAL? [Y/n]: "
        ),
        "{}",
        run.output
    );
}

/// spec: CLI-007.9
#[test]
fn a_file_that_is_not_saved_is_listed_before_the_eject_question() {
    let machine = machine_with_backup();
    let occupied = machine.backup().join("2026/2026-10-17/DSCF0002.JPG");
    fs::create_dir_all(occupied.parent().unwrap()).unwrap();
    fs::write(&occupied, b"other content").unwrap();

    let run = machine.run(&["", "n"]);

    assert!(
        run.output.contains(
            "Do not format the card: not every file is verified in 2 locations.\n\
             \x20 DCIM/100_FUJI/DSCF0002.JPG: verified in 1 of 2 locations\n\
             Eject EOS_DIGITAL? [Y/n]: "
        ),
        "{}",
        run.output
    );
}
