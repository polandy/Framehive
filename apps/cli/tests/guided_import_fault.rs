// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A guided import whose job fails in the import itself (CLI-003) against the binary
//! built with the fault-injection point (ADR 0048), on a pseudo-terminal.

#![cfg(all(target_os = "linux", feature = "fault-injection"))]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use support::guided_card::{Machine, text};
use support::terminal::run_on_terminal;

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
