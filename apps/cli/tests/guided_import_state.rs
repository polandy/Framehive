// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The state file and the sequence counter of the guided import (CLI-003) against the
//! binary on a pseudo-terminal, with a card listed in a mount table file (ADR 0050).

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use support::guided_card::{Machine, text};
use support::terminal::run_on_terminal;

const PRESET_2: &str = "{date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}";
const COUNTERS_AT_250: &str = "format_version=1\nstored_last=250\n";
const COUNTERS_AT_98: &str = "format_version=1\nstored_last=98\n";

/// spec: CLI-003.10
#[test]
fn a_state_file_of_another_version_is_ignored_with_a_warning() {
    let machine = Machine::without_card();
    let file = machine.state_file();
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(
        &file,
        format!(
            "format_version=7\ndestination={}\n",
            text(&machine.library())
        ),
    )
    .unwrap();

    let run = machine.run(&[""]);

    assert!(
        run.errors.lines().any(|line| line.starts_with("warning:")),
        "{}",
        run.errors
    );
    assert!(run.errors.contains("format_version=7"), "{}", run.errors);
    assert!(
        run.output.contains("Where should the photos go? ["),
        "{}",
        run.output
    );
}

/// spec: CLI-003.11
#[test]
fn the_state_file_has_the_lines_of_the_adr_and_reads_back() {
    let machine = Machine::new();
    machine.add_pictures(&["DSCF0001.JPG"]);
    let library = text(&machine.library());

    machine.run(&[&library, "2", "", "Harbour", "", "n"]);
    machine.add_pictures(&["DSCF0002.JPG"]);
    machine.run(&["Smith wedding", "", "n"]);

    let state = machine.state_text();
    assert!(
        state.ends_with(&format!(
            "format_version=2\ndestination={library}\ntemplate={PRESET_2}\n\
             job_code=Smith wedding\njob_code=Harbour\n"
        )),
        "{state}"
    );
    machine.add_pictures(&["DSCF0003.JPG"]);
    let reread = machine.run(&["", "", "n"]);
    assert!(
        reread.output.contains("  1  Smith wedding\n  2  Harbour\n"),
        "{}",
        reread.output
    );
    assert!(!reread.output.contains("Where should the photos go?"));
}

/// spec: CLI-003.12
#[test]
fn the_state_file_is_below_xdg_state_home_or_the_home_folder() {
    for xdg_state_home in [None, Some("relative/path")] {
        let machine = Machine::without_card();
        let mut command = machine.command();
        match xdg_state_home {
            Some(folder) => command.env("XDG_STATE_HOME", folder),
            None => command.env_remove("XDG_STATE_HOME"),
        };

        let run = run_on_terminal(command, &[""]);

        assert_eq!(run.status.code(), Some(0), "{}", run.errors);
        let file = machine
            .folders
            .home()
            .join(".local/state/framehive/guided-import.txt");
        assert!(file.is_file(), "{xdg_state_home:?}");
        assert!(!machine.root().join("relative").exists());
    }
}

/// A machine with the card `EOS_DIGITAL`, a remembered library and `template`, the
/// stored counter `counters` and two RAW files.
fn numbered(template: &str, counters: &str) -> Machine {
    let machine = Machine::new();
    machine.remember(&text(&machine.library()), Some(template), &[]);
    machine.folders.write_counters(counters);
    machine.add_raw("DSCF0001.RAF");
    machine.add_raw("DSCF0002.RAF");
    machine
}

/// spec: CLI-003.13
#[test]
fn a_stored_counter_numbers_the_files_and_moves_on() {
    let machine = numbered("{seq_stored:4}.{ext}", COUNTERS_AT_250);

    let run = machine.run(&["", "n"]);

    for name in ["0251.RAF", "0252.RAF"] {
        assert!(machine.library().join(name).is_file(), "{name}");
    }
    let counters = fs::read_to_string(machine.folders.counter_file()).unwrap();
    assert!(counters.contains("stored_last=252\n"), "{counters}");
    assert!(!run.output.contains("not recognized"), "{}", run.output);
}

/// spec: CLI-003.14
#[test]
fn numbers_that_do_not_fit_lead_to_the_template_step() {
    let machine = numbered("{seq_stored:2}.{ext}", COUNTERS_AT_98);

    let run = machine.run(&["{seq_stored:3}.{ext}", "", "q"]);

    let output = &run.output;
    let reason = output
        .find("The template cannot number EOS_DIGITAL: ")
        .unwrap_or_else(|| panic!("{output}"));
    let suggestion = output[reason..].find("{seq_stored:3}.{ext}").unwrap();
    let template_step = output[reason..]
        .find("How should the photos be named?")
        .unwrap();
    assert!(suggestion < template_step, "{output}");
    assert!(
        output[reason + template_step..].contains("  named     {seq_stored:3}.{ext}\n"),
        "{output}"
    );
}
