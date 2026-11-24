// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The backup destinations at the plan summary of the guided import (CLI-006) against
//! the binary on a pseudo-terminal, with a card listed in a mount table file (ADR 0050).

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::os::unix::fs::PermissionsExt;

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

/// spec: CLI-006.6
#[test]
fn remembered_backup_destinations_are_shown_without_asking() {
    let machine = machine_with_backups(&[]);
    let backup = text(&machine.backup());
    machine.run(&["b", &backup]);

    let run = machine.run(&[""]);

    assert!(
        run.output.contains(&format!("\x20 backup    {backup}\n")),
        "{}",
        run.output
    );
    assert!(
        !run.output.contains("Backup folder to add"),
        "{}",
        run.output
    );
}

/// spec: CLI-006.7
#[test]
fn a_state_file_of_version_1_is_read_and_written_as_version_2() {
    let machine = Machine::new();
    machine.add_pictures(&["DSCF0001.JPG"]);
    let library = text(&machine.library());
    let file = machine.state_file();
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(
        &file,
        format!(
            "format_version=1\ndestination={library}\n\
             template={{date:%Y}}/{{date:%Y-%m-%d}}/{{name}}.{{ext}}\n"
        ),
    )
    .unwrap();

    let run = machine.run(&["d", ""]);

    assert_eq!(run.errors, "");
    assert!(
        run.output.contains("\x20 backup    none\n"),
        "{}",
        run.output
    );
    assert_eq!(
        run.output.matches("Where should the photos go?").count(),
        1,
        "{}",
        run.output
    );
    assert!(
        machine.state_text().contains("format_version=2\n"),
        "{}",
        machine.state_text()
    );
}

/// spec: CLI-006.5
#[test]
fn a_failed_backup_is_reported_after_the_copied_line() {
    let machine = machine_with_backups(&[]);
    machine.add_pictures(&["DSCF0002.JPG"]);
    let backup = text(&machine.backup());
    machine.remember_backups(&text(&machine.library()), &[&backup]);
    let occupied = machine.backup().join("2026/2026-10-17/DSCF0001.JPG");
    fs::create_dir_all(occupied.parent().unwrap()).unwrap();
    fs::write(&occupied, b"other content").unwrap();

    let run = machine.run(&["", "n"]);

    let copied = "copied  DCIM/100_FUJI/DSCF0001.JPG -> 2026/2026-10-17/DSCF0001.JPG\n";
    assert!(
        run.output.contains(&format!(
            "{copied}\x20 backup failed  {backup}: name conflict\n"
        )),
        "{}",
        run.output
    );
    assert!(
        run.output.contains(&format!(
            "backup {backup}: 1 backed up, 0 already backed up, 1 failed\n"
        )),
        "{}",
        run.output
    );
    assert_eq!(fs::read(&occupied).unwrap(), b"other content");
}

/// spec: CLI-006.8
#[test]
fn a_backup_destination_that_cannot_be_created_copies_nothing() {
    let machine = machine_with_backups(&[]);
    let locked = machine.root().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
    let backup = text(&locked.join("backup"));
    machine.remember_backups(&text(&machine.library()), &[&backup]);

    let run = machine.run(&[""]);

    assert!(
        run.output.contains(&format!("Could not create {backup}: ")),
        "{}",
        run.output
    );
    assert!(
        !machine
            .library()
            .join("2026/2026-10-17/DSCF0001.JPG")
            .exists()
    );
}
