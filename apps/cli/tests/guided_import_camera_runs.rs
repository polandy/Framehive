// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! One run of the guided import per spec against the binary with its real adapters, on
//! a pseudo-terminal, with libgphoto2's `vusb` virtual camera (ADR 0034).

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::Path;

use support::guided_camera::{Machine, text};
use support::terminal::{Transcript, run_on_terminal_with};
use support::{picture, stdout};

/// The choices below the plan summary of CLI-003.
const SUMMARY_CHOICES: &str =
    "[Enter] import  [l] list  [t] template  [d] destination  [b] backup  [q] quit: ";

/// spec: CLI-014.1
#[test]
fn a_camera_appears_and_goes() {
    let machine = Machine::without_card();
    machine
        .folders
        .add("DCIM/100_FUJI/DSCF0001.JPG", &picture("DSCF0001.JPG"));
    let plug = machine.folders.root.path().join("plugged");
    let mut command = machine.camera_command();
    // The patched `vusb` driver lists the camera only while this file exists.
    command.env("VCAMERA_CONNECTED_WHILE", &plug);

    let run = run_on_terminal_with(command, |terminal| {
        terminal.type_line(&text(&machine.library()));
        terminal.wait_for("Waiting for a memory card or camera…\n");
        fs::write(&plug, "").unwrap();
        terminal.wait_for("Reading Nikon DSC D750…\n");
        terminal.type_line("");
        terminal.type_line("");
        terminal.wait_for(SUMMARY_CHOICES);
        fs::remove_file(&plug).unwrap();
        terminal.wait_for("[REMOVED] Nikon DSC D750\n");
        terminal.wait_for("Waiting for a memory card or camera…\n");
    });

    assert_eq!(run.status.code(), Some(0), "{}", run.errors);
    let output = &run.output;
    let waiting = output
        .find("Waiting for a memory card or camera…\n")
        .unwrap();
    let new = output
        .find("[NEW] Nikon DSC D750 (camera at usb:001,001)\nReading Nikon DSC D750…\n")
        .unwrap();
    let summary = output.find(SUMMARY_CHOICES).unwrap();
    let removed = output.find("[REMOVED] Nikon DSC D750\n").unwrap();
    assert!(
        waiting < new && new < summary && summary < removed,
        "{output}"
    );
    assert!(
        output[removed..].contains("Waiting for a memory card or camera…\n"),
        "{output}"
    );
    assert!(!machine.library().join("2026").exists(), "{output}");
}

/// spec: CLI-014.2
#[test]
fn a_card_and_a_camera_are_offered_to_choose_from() {
    let machine = Machine::with_card();
    machine
        .folders
        .add("DCIM/100_FUJI/DSCF0001.JPG", &picture("DSCF0001.JPG"));

    let run = machine.run_with_camera(&[&text(&machine.library()), "2"]);

    assert!(
        run.output.contains(&format!(
            "Which card or camera?\n  1  EOS_DIGITAL ({})\n  2  Nikon DSC D750 (camera at \
             usb:001,001)\n",
            text(&machine.card())
        )),
        "{}",
        run.output
    );
    assert!(
        run.output.contains("Reading Nikon DSC D750…\n"),
        "{}",
        run.output
    );
    assert!(
        !run.output.contains("Reading EOS_DIGITAL"),
        "{}",
        run.output
    );
}

/// spec: CLI-014.3
#[test]
fn a_camera_is_imported() {
    let machine = Machine::without_card();
    for name in ["DSCF0001.JPG", "DSCF0002.JPG"] {
        machine
            .folders
            .add(&format!("DCIM/100_FUJI/{name}"), &picture(name));
    }

    let run = machine.run_with_camera(&[&text(&machine.library()), "", "", "", "y"]);

    assert!(
        run.output.contains(
            "2 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n\
             The camera can be formatted: every file is verified in 1 location.\n\
             Only the files below DCIM were checked. Files outside DCIM, such as a phone's \
             other folders, are not backed up.\n"
        ),
        "{}",
        run.output
    );
    assert!(!run.output.contains("on the card"), "{}", run.output);
    for name in ["DSCF0001.JPG", "DSCF0002.JPG"] {
        let target = Path::new("2026/2026-10-17").join(name);
        assert!(machine.library().join(target).is_file(), "{}", run.output);
    }
    let log = machine.folders.import_log(&["1"]);
    assert!(
        stdout(&log).contains("source               camera:Nikon DSC D750\n"),
        "{}",
        stdout(&log)
    );
}

/// spec: CLI-014.11
#[test]
fn a_camera_with_nothing_to_import_is_reported() {
    let machine = Machine::without_card();
    fs::create_dir_all(machine.folders.source().join("DCIM")).unwrap();

    let run = machine.run_with_camera(&[&text(&machine.library())]);

    assert!(
        run.output.contains(
            "Reading Nikon DSC D750…\n\
             Nikon DSC D750 has no photos or videos to import.\n\
             Waiting for a memory card or camera…\n"
        ),
        "{}",
        run.output
    );
    assert!(!machine.library().join("2026").exists(), "{}", run.output);
}

/// Runs a camera import of one photo, answers `answer` to the release question and
/// reports whether the camera is free once the dialog waits again.
fn release_after_import(machine: &Machine, answer: &str) -> (Transcript, bool) {
    machine
        .folders
        .add("DCIM/100_FUJI/DSCF0001.JPG", &picture("DSCF0001.JPG"));
    let mut free = false;
    let run = run_on_terminal_with(machine.camera_command(), |terminal| {
        for line in [text(&machine.library()).as_str(), "", "", ""] {
            terminal.type_line(line);
        }
        terminal.wait_for("Release Nikon DSC D750? [Y/n]: ");
        terminal.type_line(answer);
        terminal.wait_for("Waiting for a memory card or camera…\n");
        // The patched `vusb` driver locks the folder while the camera is open.
        free = fs::File::open(machine.folders.source())
            .unwrap()
            .try_lock()
            .is_ok();
    });
    assert_eq!(run.status.code(), Some(0), "{}", run.errors);
    assert_eq!(
        run.output.matches("Reading Nikon").count(),
        1,
        "{}",
        run.output
    );
    (run, free)
}

/// spec: CLI-014.7
#[test]
fn a_camera_is_released_on_yes() {
    let machine = Machine::without_card();

    let (run, free) = release_after_import(&machine, "y");

    assert!(
        run.output.contains(
            "Release Nikon DSC D750? [Y/n]: Nikon DSC D750 can be disconnected now.\n\
             Waiting for a memory card or camera…\n"
        ),
        "{}",
        run.output
    );
    assert!(free, "{}", run.output);
}

/// spec: CLI-014.8
#[test]
fn a_camera_is_kept_open_on_no() {
    let machine = Machine::without_card();

    let (run, free) = release_after_import(&machine, "n");

    assert!(
        run.output
            .contains("Release Nikon DSC D750? [Y/n]: Waiting for a memory card or camera…\n"),
        "{}",
        run.output
    );
    assert!(!free, "{}", run.output);
}

/// spec: CLI-014.8
#[test]
fn a_camera_kept_open_is_released_once_it_is_disconnected() {
    let machine = Machine::without_card();
    machine
        .folders
        .add("DCIM/100_FUJI/DSCF0001.JPG", &picture("DSCF0001.JPG"));
    let plug = machine.folders.root.path().join("plugged");
    fs::write(&plug, "").unwrap();
    let mut command = machine.camera_command();
    command.env("VCAMERA_CONNECTED_WHILE", &plug);
    let mut free = Vec::new();

    let run = run_on_terminal_with(command, |terminal| {
        for line in [text(&machine.library()).as_str(), "", "", ""] {
            terminal.type_line(line);
        }
        terminal.wait_for("Release Nikon DSC D750? [Y/n]: ");
        terminal.type_line("n");
        terminal.wait_for("Waiting for a memory card or camera…\n");
        let is_free = || {
            fs::File::open(machine.folders.source())
                .unwrap()
                .try_lock()
                .is_ok()
        };
        free.push(is_free());
        fs::remove_file(&plug).unwrap();
        terminal.wait_for("[REMOVED] Nikon DSC D750\n");
        free.push(is_free());
    });

    assert_eq!(run.status.code(), Some(0), "{}", run.errors);
    assert_eq!(free, [false, true], "{}", run.output);
    assert_eq!(
        run.output.matches("Reading Nikon").count(),
        1,
        "{}",
        run.output
    );
}

/// spec: CLI-014.9
#[test]
fn a_camera_disconnected_during_the_import_is_resumed() {
    let machine = Machine::without_card();
    for name in ["DSCF0001.JPG", "DSCF0002.JPG", "DSCF0003.JPG"] {
        machine
            .folders
            .add(&format!("DCIM/100_FUJI/{name}"), &picture(name));
    }
    let day = machine.library().join("2026/2026-10-17");
    let plug = machine.folders.root.path().join("plugged");
    fs::write(&plug, "").unwrap();
    let mut command = machine.camera_command();
    // The patched `vusb` driver removes `plug` once the first copy exists, and answers again once it is back.
    command
        .env("VCAMERA_UNPLUG_WHEN", day.join("DSCF0001.JPG"))
        .env("VCAMERA_CONNECTED_WHILE", &plug);

    let run = run_on_terminal_with(command, |terminal| {
        for line in [text(&machine.library()).as_str(), "", "", ""] {
            terminal.type_line(line);
        }
        terminal.wait_for(
            "[REMOVED] Nikon DSC D750\n\
             Nikon DSC D750 was disconnected during the import. Connect it again to go on.\n",
        );
        // The dialog answers a line only after a poll that no longer lists the camera.
        terminal.type_line("");
        terminal.wait_for("Insert a memory card or connect a camera, or type q to quit.\n");
        fs::write(&plug, "").unwrap();
        terminal.wait_for("Release Nikon DSC D750? [Y/n]: ");
        terminal.type_line("y");
        terminal.wait_for("Waiting for a memory card or camera…\n");
    });

    assert_eq!(run.status.code(), Some(0), "{}", run.errors);
    let output = &run.output;
    let resumed = &output[output.find("[REMOVED] Nikon DSC D750\n").unwrap()..];
    assert!(
        resumed.contains("[NEW] Nikon DSC D750 (camera at usb:001,001)\nReading Nikon DSC D750…\n"),
        "{output}"
    );
    assert!(resumed.contains("2 copied, 1 already imported"), "{output}");
    assert!(!resumed.contains(SUMMARY_CHOICES), "{output}");
    for name in ["DSCF0001.JPG", "DSCF0002.JPG", "DSCF0003.JPG"] {
        assert_eq!(fs::read(day.join(name)).unwrap(), picture(name), "{output}");
    }
}

/// spec: CLI-014.10
#[test]
fn a_camera_that_stops_answering_stops_the_import() {
    let machine = Machine::without_card();
    for name in ["DSCF0001.JPG", "DSCF0002.JPG", "DSCF0003.JPG"] {
        machine
            .folders
            .add(&format!("DCIM/100_FUJI/{name}"), &picture(name));
    }
    let first = machine.library().join("2026/2026-10-17/DSCF0001.JPG");
    let mut command = machine.camera_command();
    // The patched `vusb` driver stops answering once the first copy exists, but stays listed.
    command.env("VCAMERA_UNPLUG_WHEN", &first);
    let mut free = false;

    let run = run_on_terminal_with(command, |terminal| {
        for line in [text(&machine.library()).as_str(), "", "", ""] {
            terminal.type_line(line);
        }
        terminal.wait_for(
            "[REMOVED] Nikon DSC D750\n\
             Nikon DSC D750 was disconnected during the import. Connect it again to go on.\n",
        );
        // The dialog answers a line only once it waits again, after the import let go.
        terminal.type_line("");
        terminal.wait_for("Insert a memory card or connect a camera, or type q to quit.\n");
        free = fs::File::open(machine.folders.source())
            .unwrap()
            .try_lock()
            .is_ok();
    });

    assert_eq!(run.status.code(), Some(0), "{}", run.errors);
    assert!(free, "{}", run.output);
    assert_eq!(
        run.output.matches("[REMOVED] Nikon DSC D750\n").count(),
        1,
        "{}",
        run.output
    );
    assert!(first.is_file(), "{}", run.output);
    assert!(
        !machine
            .library()
            .join("2026/2026-10-17/DSCF0003.JPG")
            .exists(),
        "{}",
        run.output
    );
}
