// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Runs of the guided import against the binary with a camera that cannot be read at
//! first, a camera in use or one without a storage, through libgphoto2's `vusb` virtual
//! camera (ADR 0034).

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use support::guided_camera::{Machine, text};
use support::picture;
use support::terminal::run_on_terminal_with;

/// spec: CLI-014.4
#[test]
fn a_camera_in_use_is_tried_again() {
    let machine = Machine::without_card();
    machine
        .folders
        .add("DCIM/100_FUJI/DSCF0001.JPG", &picture("DSCF0001.JPG"));

    let run = machine.run_with_held_camera(&text(&machine.library()), &["", "", "", "y"]);

    assert!(
        run.output.contains(
            "The camera Nikon DSC D750 at usb:001,001 is in use by another program; close it \
             (the file manager on Linux, ptpcamerad on macOS) and try again.\n\
             Try again? [Y/n]: "
        ),
        "{}",
        run.output
    );
    assert!(run.output.contains("1 copied"), "{}", run.output);
    assert!(
        machine
            .library()
            .join("2026/2026-10-17/DSCF0001.JPG")
            .is_file(),
        "{}",
        run.output
    );
}

/// spec: CLI-014.5
#[test]
fn a_camera_in_use_is_not_offered_again_while_it_stays_connected() {
    let machine = Machine::without_card();
    machine
        .folders
        .add("DCIM/100_FUJI/DSCF0001.JPG", &picture("DSCF0001.JPG"));

    let run = machine.run_with_holder(&text(&machine.library()), |terminal, holder| {
        terminal.wait_for("Try again? [Y/n]: ");
        terminal.type_line("n");
        terminal.wait_for("Waiting for a memory card or camera…\n");
        // The camera is free now, but the dialog waits until it is connected again.
        drop(holder);
    });

    assert_eq!(
        run.output.matches("Try again?").count(),
        1,
        "{}",
        run.output
    );
    assert!(!run.output.contains("Reading Nikon"), "{}", run.output);
    assert!(!machine.library().join("2026").exists(), "{}", run.output);
}

/// spec: CLI-014.5
#[test]
fn a_camera_in_use_is_offered_again_once_connected_again() {
    let machine = Machine::without_card();
    machine
        .folders
        .add("DCIM/100_FUJI/DSCF0001.JPG", &picture("DSCF0001.JPG"));
    let plug = machine.folders.root.path().join("plugged");
    fs::write(&plug, "").unwrap();
    let mut command = machine.camera_command();
    // The patched `vusb` driver lists the camera only while this file exists.
    command.env("VCAMERA_CONNECTED_WHILE", &plug);
    // The patched `vusb` driver reports its camera as claimed while the folder is locked.
    let holder = fs::File::open(machine.folders.source()).unwrap();
    holder.try_lock().unwrap();

    let run = run_on_terminal_with(command, |terminal| {
        terminal.type_line(&text(&machine.library()));
        terminal.wait_for("Try again? [Y/n]: ");
        terminal.type_line("n");
        terminal.wait_for("Waiting for a memory card or camera…\n");
        drop(holder);
        fs::remove_file(&plug).unwrap();
        terminal.wait_for("[REMOVED] Nikon DSC D750\n");
        fs::write(&plug, "").unwrap();
        terminal.wait_for("Reading Nikon DSC D750…\n");
        terminal.type_line("");
        terminal.type_line("");
        terminal.wait_for("[Enter] import");
    });

    assert_eq!(run.status.code(), Some(0), "{}", run.errors);
    let output = &run.output;
    let removed = output.find("[REMOVED] Nikon DSC D750\n").unwrap();
    assert_eq!(output.matches("Try again?").count(), 1, "{output}");
    assert!(
        output[removed..]
            .contains("[NEW] Nikon DSC D750 (camera at usb:001,001)\nReading Nikon DSC D750…\n"),
        "{output}"
    );
}

/// spec: CLI-014.6
#[test]
fn a_camera_without_a_storage_is_tried_again() {
    let machine = Machine::without_card();
    machine
        .folders
        .add("DCIM/100_FUJI/DSCF0001.JPG", &picture("DSCF0001.JPG"));
    let lock = machine.folders.root.path().join("locked");
    fs::write(&lock, "").unwrap();
    let mut command = machine.camera_command();
    // The patched `vusb` driver lists no storage while this file exists.
    command.env("VCAMERA_LOCKED_WHILE", &lock);

    let run = run_on_terminal_with(command, |terminal| {
        terminal.type_line(&text(&machine.library()));
        terminal.wait_for("Try again? [Y/n]: ");
        fs::remove_file(&lock).unwrap();
        for line in ["y", "", "", "", "y"] {
            terminal.type_line(line);
        }
    });

    assert_eq!(run.status.code(), Some(0), "{}", run.errors);
    assert!(
        run.output.contains(
            "The camera Nikon DSC D750 at usb:001,001 shows no storage; unlock it and choose \
             file transfer (MTP or PTP) on the device, then try again.\n\
             Try again? [Y/n]: "
        ),
        "{}",
        run.output
    );
    assert!(run.output.contains("1 copied"), "{}", run.output);
    assert!(
        machine
            .library()
            .join("2026/2026-10-17/DSCF0001.JPG")
            .is_file(),
        "{}",
        run.output
    );
}
