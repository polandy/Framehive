// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! One run of the guided import per spec against the binary with its real adapters, on
//! a pseudo-terminal, with a card listed in a mount table file (ADR 0050) or libgphoto2's
//! `vusb` virtual camera (ADR 0034).

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use support::terminal::{Transcript, run_on_terminal, run_on_terminal_with};
use support::{Folders, iolib_folder, picture, stdout};

const CARD: &str = "EOS_DIGITAL";

/// A sidecar on the card with the creator `Camera Owner`.
const CAMERA_SIDECAR: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:creator><rdf:Seq><rdf:li>Camera Owner</rdf:li></rdf:Seq></dc:creator></rdf:Description></rdf:RDF></x:xmpmeta>"#;

/// The card `EOS_DIGITAL` below a media folder, listed in a mount table file, and the
/// folders of the runs, all below a temporary root.
struct Machine {
    folders: Folders,
}

impl Machine {
    fn new() -> Self {
        let machine = Machine::without_card();
        fs::create_dir_all(machine.card().join("DCIM")).unwrap();
        let line = format!(
            "100 1 8:17 / {} rw,relatime - vfat /dev/sdz1 rw\n",
            machine.card().display()
        );
        fs::write(machine.mountinfo(), line).unwrap();
        machine
    }

    fn without_card() -> Self {
        let machine = Machine {
            folders: Folders::new(),
        };
        fs::write(machine.mountinfo(), "").unwrap();
        machine
    }

    fn root(&self) -> &Path {
        self.folders.root.path()
    }

    fn media(&self) -> PathBuf {
        self.root().join("media")
    }

    fn card(&self) -> PathBuf {
        self.media().join(CARD)
    }

    fn mountinfo(&self) -> PathBuf {
        self.root().join("mountinfo")
    }

    fn library(&self) -> PathBuf {
        self.folders.destination()
    }

    fn backup(&self) -> PathBuf {
        self.root().join("backup")
    }

    /// Adds pictures captured on 2026-10-17 to `DCIM/100_FUJI` of the card.
    fn add_pictures(&self, names: &[&str]) {
        for name in names {
            self.add_to_card(&format!("DCIM/100_FUJI/{name}"), &picture(name));
        }
    }

    fn add_to_card(&self, relative: &str, content: &[u8]) {
        let path = self.card().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// Runs `framehive` on a terminal with the card's mount table, typing `script`.
    fn run(&self, script: &[&str]) -> Transcript {
        finish(self.command(), script)
    }

    /// Like [`Machine::run`], with `vusb` serving the source folder as the only camera.
    fn run_with_camera(&self, script: &[&str]) -> Transcript {
        let mut command = self.command();
        command
            .env("IOLIBS", iolib_folder("vusb"))
            .env("VCAMERADIR", self.folders.source());
        finish(command, script)
    }

    /// Like [`Machine::run_with_camera`], with another program holding the camera until
    /// the dialog asks to try again.
    fn run_with_held_camera(&self, library: &str, rest: &[&str]) -> Transcript {
        let mut command = self.command();
        command
            .env("IOLIBS", iolib_folder("vusb"))
            .env("VCAMERADIR", self.folders.source());
        // The patched `vusb` driver reports its camera as claimed while the folder is locked.
        let holder = fs::File::open(self.folders.source()).unwrap();
        holder.try_lock().unwrap();
        let transcript = run_on_terminal_with(command, |terminal| {
            terminal.type_line(library);
            terminal.wait_for("Try again? [Y/n]: ");
            drop(holder);
            terminal.type_line("y");
            for line in rest {
                terminal.type_line(line);
            }
        });
        assert_eq!(transcript.status.code(), Some(0), "{}", transcript.errors);
        transcript
    }

    fn command(&self) -> Command {
        let mut command = self.folders.framehive_command(&[]);
        command
            .env("FRAMEHIVE_MOUNTINFO", self.mountinfo())
            .env("FRAMEHIVE_MEDIA_FOLDERS", self.media())
            .current_dir(self.root());
        command
    }
}

fn finish(command: Command, script: &[&str]) -> Transcript {
    let transcript = run_on_terminal(command, script);
    assert_eq!(transcript.status.code(), Some(0), "{}", transcript.errors);
    transcript
}

fn text(path: &Path) -> String {
    path.display().to_string()
}

/// spec: CLI-003.9
#[test]
fn a_second_run_asks_only_for_the_job_code() {
    let machine = Machine::new();
    machine.add_pictures(&["DSCF0001.JPG"]);
    let library = text(&machine.library());

    let first = machine.run(&[&library, "2", "", "Harbour", "", "n"]);
    machine.add_pictures(&["DSCF0002.JPG"]);
    let second = machine.run(&["", "", "n"]);

    assert!(
        first.output.contains("Where should the photos go?"),
        "{}",
        first.output
    );
    assert!(
        first.output.contains("\n1 copied, 0 already imported"),
        "{}",
        first.output
    );
    assert!(
        !second.output.contains("Where should the photos go?"),
        "{}",
        second.output
    );
    assert!(
        !second.output.contains("How should the photos be named?"),
        "{}",
        second.output
    );
    assert!(second.output.contains("  1  Harbour"), "{}", second.output);
    assert!(
        second.output.contains("Choose 1 or type a job code [1]: "),
        "{}",
        second.output
    );
    assert!(
        second.output.contains("\n1 copied, 1 already imported"),
        "{}",
        second.output
    );
    for name in ["DSCF0001.JPG", "DSCF0002.JPG"] {
        assert!(
            machine
                .library()
                .join("2026/2026-10-17_Harbour")
                .join(name)
                .is_file()
        );
    }
}

/// spec: CLI-006.4
#[test]
fn a_backup_destination_gets_every_file() {
    let machine = Machine::new();
    machine.add_pictures(&["DSCF0001.JPG", "DSCF0002.JPG"]);
    let backup = text(&machine.backup());

    let run = machine.run(&[&text(&machine.library()), "", "", "b", &backup, "", "n"]);

    assert!(
        run.output.contains(&format!("  backup    {backup}\n")),
        "{}",
        run.output
    );
    assert!(
        run.output.contains(&format!(
            "backup {backup}: 2 backed up, 0 already backed up, 0 failed\n"
        )),
        "{}",
        run.output
    );
    for name in ["DSCF0001.JPG", "DSCF0002.JPG"] {
        let target = Path::new("2026/2026-10-17").join(name);
        assert!(machine.library().join(&target).is_file());
        assert!(machine.backup().join(&target).is_file());
    }
}

/// spec: CLI-011.12
#[test]
fn a_backup_on_the_file_system_of_the_destination_is_warned_about() {
    let machine = Machine::new();
    machine.add_pictures(&["DSCF0001.JPG"]);
    let (library, backup) = (text(&machine.library()), text(&machine.backup()));

    let run = machine.run(&[&library, "", "", "b", &backup, "", "n"]);

    let warning = format!(
        "backup {backup} is on the same file system as {library}; it does not protect against losing that disk"
    );
    assert_eq!(run.errors.matches(&warning).count(), 1, "{}", run.errors);
    let target = Path::new("2026/2026-10-17/DSCF0001.JPG");
    assert!(machine.library().join(target).is_file());
    assert!(machine.backup().join(target).is_file());
}

/// spec: CLI-017.6
#[test]
fn the_report_shows_a_kept_creator_and_the_sidecar_count() {
    let machine = Machine::new();
    machine.add_pictures(&["DSCF0001.JPG"]);
    machine.add_to_card("DCIM/100_FUJI/DSCF0001.JPG.xmp", CAMERA_SIDECAR.as_bytes());
    let configuration = machine.folders.configuration_folder();
    fs::create_dir_all(&configuration).unwrap();
    fs::write(
        configuration.join("config.yaml"),
        "schema_version: 1\nmetadata:\n  creator: [Andy Pollari]\n",
    )
    .unwrap();

    let run = machine.run(&[&text(&machine.library()), "", "", "", "n"]);

    assert!(
        run.output.contains(
            "copied  DCIM/100_FUJI/DSCF0001.JPG -> 2026/2026-10-17/DSCF0001.JPG\n  \
             kept creator  Camera Owner (not Andy Pollari)\n"
        ),
        "{}",
        run.output
    );
    assert!(
        run.output.contains("0 left out\n1 sidecars written\n"),
        "{}",
        run.output
    );
    assert!(
        machine
            .library()
            .join("2026/2026-10-17/DSCF0001.JPG.xmp")
            .is_file()
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
