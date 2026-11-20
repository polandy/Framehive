// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The plan summary of the guided import (CLI-003) against the binary on a
//! pseudo-terminal, with a card listed in a mount table file (ADR 0050).

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use support::guided_card::{Machine, text};
use support::{photo_taken_at, picture};

const PRESET_1: &str = "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}";
const PRESET_2: &str = "{date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}";

/// A machine with the card `EOS_DIGITAL`, a remembered library and `template`, and
/// `DSCF0001.JPG` captured on 2026-10-17 and `DSCF0002.JPG` on 2026-10-19.
fn two_days(template: &str, job_codes: &[&str]) -> Machine {
    let machine = Machine::new();
    machine.remember(&text(&machine.library()), Some(template), job_codes);
    machine.add_to_card("DCIM/100_FUJI/DSCF0001.JPG", &picture("DSCF0001.JPG"));
    let mut second = photo_taken_at("2026:10:19 09:00:00");
    second.extend(b"DSCF0002.JPG");
    machine.add_to_card("DCIM/100_FUJI/DSCF0002.JPG", &second);
    machine
}

/// Makes the file `relative` of the card report `size` bytes by extending it with a hole.
fn pretend_size(machine: &Machine, relative: &str, size: u64) {
    let file = fs::OpenOptions::new()
        .write(true)
        .open(machine.card().join(relative))
        .unwrap();
    file.set_len(size).unwrap();
}

/// spec: CLI-003.1
#[test]
fn the_summary_shows_files_size_dates_destination_template_and_job_code() {
    let machine = two_days(PRESET_2, &["Smith wedding"]);
    pretend_size(&machine, "DCIM/100_FUJI/DSCF0001.JPG", 1_500_000_000);
    pretend_size(&machine, "DCIM/100_FUJI/DSCF0002.JPG", 1_500_000_000);

    let run = machine.run(&[""]);

    let expected = format!(
        "EOS_DIGITAL: 2 files, 3.0 GB, captured 2026-10-17 to 2026-10-19\n\
         \x20 to        {}\n\
         \x20 backup    none\n\
         \x20 named     {PRESET_2}\n\
         \x20 job code  Smith wedding\n\
         [Enter] import  [l] list  [j] job code  [t] template  [d] destination  [b] backup  [q] quit: ",
        text(&machine.library())
    );
    assert!(run.output.contains(&expected), "{}", run.output);
}

/// spec: CLI-003.2
#[test]
fn listing_prints_the_dry_run_lines_and_copies_nothing() {
    let machine = two_days(PRESET_1, &[]);
    let imported = machine.library().join("2026/2026-10-17/DSCF0001.JPG");
    fs::create_dir_all(imported.parent().unwrap()).unwrap();
    fs::write(&imported, picture("DSCF0001.JPG")).unwrap();

    let run = machine.run(&["l"]);

    let listed = run.output.find("[q] quit: ").expect(&run.output);
    let listing = &run.output[listed..];
    assert!(
        listing.contains(
            "already imported  DCIM/100_FUJI/DSCF0001.JPG -> 2026/2026-10-17/DSCF0001.JPG\n\
             would copy  DCIM/100_FUJI/DSCF0002.JPG -> 2026/2026-10-19/DSCF0002.JPG"
        ),
        "{}",
        run.output
    );
    assert!(
        listing.contains(
            "\n1 would copy, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n\
             EOS_DIGITAL: 2 files, "
        ),
        "{}",
        run.output
    );
    assert!(!machine.library().join("2026/2026-10-19").exists());
}

/// spec: CLI-003.3
#[test]
fn the_destination_can_be_changed_at_the_summary() {
    let machine = two_days(PRESET_1, &[]);
    let archive = text(&machine.root().join("Archive"));

    let run = machine.run(&["d", &archive]);

    assert!(
        run.output.contains(&format!(
            "Where should the photos go? [{}]: EOS_DIGITAL: 2 files, ",
            text(&machine.library())
        )),
        "{}",
        run.output
    );
    assert!(
        run.output.contains(&format!("\x20 to        {archive}\n")),
        "{}",
        run.output
    );
    assert!(
        machine
            .state_text()
            .contains(&format!("destination={archive}\n")),
        "{}",
        machine.state_text()
    );
}
