// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A machine for runs of the guided import with libgphoto2's `vusb` virtual camera
//! serving its source folder (ADR 0034).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::terminal::{Terminal, Transcript, run_on_terminal, run_on_terminal_with};
use super::{Folders, iolib_folder};

/// The source folder served as the only camera, optionally the card `EOS_DIGITAL`, and
/// the folders of the runs, all below a temporary root.
pub(crate) struct Machine {
    pub(crate) folders: Folders,
}

impl Machine {
    pub(crate) fn without_card() -> Self {
        let machine = Machine {
            folders: Folders::new(),
        };
        fs::write(machine.mountinfo(), "").unwrap();
        machine
    }

    pub(crate) fn with_card() -> Self {
        let machine = Machine::without_card();
        fs::create_dir_all(machine.card().join("DCIM")).unwrap();
        let line = format!(
            "100 1 8:17 / {} rw,relatime - vfat /dev/sdz1 rw\n",
            machine.card().display()
        );
        fs::write(machine.mountinfo(), line).unwrap();
        machine
    }

    pub(crate) fn card(&self) -> PathBuf {
        self.folders.root.path().join("media/EOS_DIGITAL")
    }

    pub(crate) fn mountinfo(&self) -> PathBuf {
        self.folders.root.path().join("mountinfo")
    }

    pub(crate) fn library(&self) -> PathBuf {
        self.folders.destination()
    }

    /// Runs `framehive` on a terminal with `vusb` serving the source folder, typing `script`.
    pub(crate) fn run_with_camera(&self, script: &[&str]) -> Transcript {
        run_on_terminal(self.camera_command(), script)
    }

    /// Like [`Machine::run_with_camera`], with another program holding the camera until
    /// the dialog asks to try again.
    pub(crate) fn run_with_held_camera(&self, library: &str, rest: &[&str]) -> Transcript {
        self.run_with_holder(library, |terminal, holder| {
            terminal.wait_for("Try again? [Y/n]: ");
            drop(holder);
            terminal.type_line("y");
            for line in rest {
                terminal.type_line(line);
            }
        })
    }

    /// Runs with another program holding the camera, which `act` types around and may
    /// release by dropping the holder.
    pub(crate) fn run_with_holder(
        &self,
        library: &str,
        act: impl FnOnce(&mut Terminal, fs::File),
    ) -> Transcript {
        // The patched `vusb` driver reports its camera as claimed while the folder is locked.
        let holder = fs::File::open(self.folders.source()).unwrap();
        holder.try_lock().unwrap();
        let transcript = run_on_terminal_with(self.camera_command(), |terminal| {
            terminal.type_line(library);
            act(terminal, holder);
        });
        assert_eq!(transcript.status.code(), Some(0), "{}", transcript.errors);
        transcript
    }

    pub(crate) fn camera_command(&self) -> Command {
        let mut command = self.folders.framehive_command(&[]);
        command
            .env("FRAMEHIVE_MOUNTINFO", self.mountinfo())
            .env(
                "FRAMEHIVE_MEDIA_FOLDERS",
                self.folders.root.path().join("media"),
            )
            .env("IOLIBS", iolib_folder("vusb"))
            .env("VCAMERADIR", self.folders.source())
            .current_dir(self.folders.root.path());
        command
    }
}

pub(crate) fn text(path: &Path) -> String {
    path.display().to_string()
}
