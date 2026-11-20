// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import, progress and eject of the guided import (CLI-003) against the binary on
//! a pseudo-terminal, with a card listed in a mount table file and a stand-in for
//! `udisksctl` (ADR 0050).

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use support::guided_card::{Machine, text};

const PRESET_1: &str = "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}";
const WAITING: &str = "Waiting for a memory card or camera…";

/// A machine with the card `EOS_DIGITAL` holding `names`, and a remembered library.
fn card_with(names: &[&str]) -> Machine {
    let machine = Machine::new();
    machine.remember(&text(&machine.library()), Some(PRESET_1), &[]);
    machine.add_pictures(names);
    machine
}

/// The stand-in for `udisksctl` of an eject that works: the card leaves the mount table.
fn ejecting() -> &'static str {
    ": > \"$FRAMEHIVE_MOUNTINFO\""
}

/// spec: CLI-003.4
#[test]
fn a_destination_on_the_card_does_not_start() {
    let machine = Machine::new();
    let on_card = text(&machine.card().join("import"));
    machine.remember(&on_card, Some(PRESET_1), &[]);
    machine.add_pictures(&["DSCF0001.JPG"]);

    let run = machine.run(&[""]);

    assert!(
        run.output.contains(&format!(
            "\x20 to        {on_card}\n\
             \x20 backup    none\n\
             \x20 named     {PRESET_1}\n\
             The destination is on the card; type d to choose another.\n"
        )),
        "{}",
        run.output
    );
    assert!(!run.output.contains(" copied, "), "{}", run.output);
    assert!(!machine.card().join("import").exists());
}

/// The clock of the binary is the system clock, so the rate is not fixed; the dialog
/// tests check it with a clock that advances one second per file.
/// spec: CLI-003.5
#[test]
fn the_progress_line_is_redrawn_after_each_file_and_the_report_follows() {
    let names = ["DSCF0001.JPG", "DSCF0002.JPG", "DSCF0003.JPG"];
    let machine = card_with(&names);
    for name in names {
        let file = fs::OpenOptions::new()
            .write(true)
            .open(machine.card().join("DCIM/100_FUJI").join(name))
            .unwrap();
        file.set_len(10_000_000).unwrap();
    }

    let run = machine.run(&["", "n"]);

    for drawn in [
        "\r  0/3 files  0.0/30.0 MB  -- MB/s  --:-- left",
        "\r  1/3 files  10.0/30.0 MB  ",
        "\r  2/3 files  20.0/30.0 MB  ",
    ] {
        assert!(run.output.contains(drawn), "{drawn:?} in {}", run.output);
    }
    let last = run
        .output
        .rfind("\r  3/3 files  30.0/30.0 MB  ")
        .expect(&run.output);
    let line = run.output[last + 1..].lines().next().unwrap().trim_end();
    let rate = line.strip_prefix("  3/3 files  30.0/30.0 MB  ").unwrap();
    let megabytes = rate.strip_suffix(" MB/s  0:00 left").expect(line);
    assert!(megabytes.parse::<u64>().is_ok(), "{line}");
    assert!(
        run.output[last..].contains("\n3 copied, 0 already imported"),
        "{}",
        run.output
    );
    for name in names {
        let copied = machine.library().join("2026/2026-10-17").join(name);
        assert_eq!(fs::metadata(copied).unwrap().len(), 10_000_000);
    }
}

/// spec: CLI-003.6
#[test]
fn an_ejected_card_can_be_removed() {
    let machine = card_with(&["DSCF0001.JPG"]);

    let run = machine.run_with_udisksctl(ejecting(), &["", ""]);

    assert!(
        run.output.contains(&format!(
            "Eject EOS_DIGITAL? [Y/n]: EOS_DIGITAL can be removed now.\n{WAITING}"
        )),
        "{}",
        run.output
    );
    assert_eq!(
        machine.eject_arguments(),
        "unmount --block-device /dev/sdz1 --no-user-interaction\n"
    );
}

/// The reason is what `udisksctl` wrote to standard error, after its exit status.
/// spec: CLI-003.7
#[test]
fn a_failed_eject_is_reported_and_the_dialog_waits() {
    let machine = card_with(&["DSCF0001.JPG"]);

    let run = machine.run_with_udisksctl("echo 'target is busy' >&2; exit 1", &["", "y"]);

    let asked = run
        .output
        .find("Eject EOS_DIGITAL? [Y/n]: ")
        .expect(&run.output);
    let after = &run.output[asked + "Eject EOS_DIGITAL? [Y/n]: ".len()..];
    let reported = after.lines().next().unwrap();
    assert!(
        reported.starts_with("Could not eject EOS_DIGITAL: udisksctl failed with exit status: 1")
            && reported.ends_with(": target is busy"),
        "{reported}"
    );
    assert!(after.contains(WAITING), "{}", run.output);
}
