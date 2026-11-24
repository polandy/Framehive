// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A machine for runs of the guided import with a card listed in a mount table file
//! (ADR 0050).

use std::fmt::Write;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::terminal::{Transcript, run_on_terminal};
use metadata::testing::raf;

use super::{Folders, picture};

const CARD: &str = "EOS_DIGITAL";

/// The card `EOS_DIGITAL` below a media folder, listed in a mount table file, and the
/// folders of the runs, all below a temporary root.
pub(crate) struct Machine {
    pub(crate) folders: Folders,
}

impl Machine {
    pub(crate) fn new() -> Self {
        let machine = Machine::without_card();
        machine.set_cards(&[CARD]);
        machine
    }

    pub(crate) fn without_card() -> Self {
        let machine = Machine {
            folders: Folders::new(),
        };
        fs::write(machine.mountinfo(), "").unwrap();
        machine
    }

    /// Mounts exactly the cards `names` below the media folder, each with a `DCIM`
    /// folder; the mount table is replaced whole, as a poll must not see it half written.
    pub(crate) fn set_cards(&self, names: &[&str]) {
        let mut table = String::new();
        for name in names {
            let card = self.media().join(name);
            fs::create_dir_all(card.join("DCIM")).unwrap();
            writeln!(
                table,
                "100 1 8:17 / {} rw,relatime - vfat /dev/sdz1 rw",
                card.display()
            )
            .unwrap();
        }
        let next = self.root().join("mountinfo.next");
        fs::write(&next, table).unwrap();
        fs::rename(next, self.mountinfo()).unwrap();
    }

    /// Writes the remembered answers (ADR 0025): the destination, the template and the
    /// job codes, most recent first.
    pub(crate) fn remember(&self, destination: &str, template: Option<&str>, job_codes: &[&str]) {
        let mut lines = vec![
            "format_version=2".to_owned(),
            format!("destination={destination}"),
        ];
        lines.extend(template.map(|template| format!("template={template}")));
        lines.extend(
            job_codes
                .iter()
                .map(|job_code| format!("job_code={job_code}")),
        );
        let text = lines.join("\n") + "\n";
        let file = self.state_file();
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }

    /// Writes the remembered destination, the template `By day` and the backup
    /// destinations (ADR 0030).
    pub(crate) fn remember_backups(&self, destination: &str, backups: &[&str]) {
        let mut text = format!(
            "format_version=2\ndestination={destination}\n\
             template={{date:%Y}}/{{date:%Y-%m-%d}}/{{name}}.{{ext}}\n"
        );
        for backup in backups {
            writeln!(text, "backup={backup}").unwrap();
        }
        let file = self.state_file();
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }

    pub(crate) fn state_file(&self) -> PathBuf {
        self.root().join("state/framehive/guided-import.txt")
    }

    /// The remembered answers as the state file holds them.
    pub(crate) fn state_text(&self) -> String {
        fs::read_to_string(self.state_file()).unwrap_or_default()
    }

    /// Adds a Fuji RAW captured on 2026-10-17 to `DCIM/100_FUJI` of the card.
    pub(crate) fn add_raw(&self, name: &str) {
        self.add_to_card(&format!("DCIM/100_FUJI/{name}"), &raf(&picture(name)));
    }

    pub(crate) fn root(&self) -> &Path {
        self.folders.root.path()
    }

    pub(crate) fn media(&self) -> PathBuf {
        self.root().join("media")
    }

    pub(crate) fn card(&self) -> PathBuf {
        self.media().join(CARD)
    }

    pub(crate) fn mountinfo(&self) -> PathBuf {
        self.root().join("mountinfo")
    }

    pub(crate) fn library(&self) -> PathBuf {
        self.folders.destination()
    }

    pub(crate) fn backup(&self) -> PathBuf {
        self.root().join("backup")
    }

    /// Adds pictures captured on 2026-10-17 to `DCIM/100_FUJI` of the card.
    pub(crate) fn add_pictures(&self, names: &[&str]) {
        for name in names {
            self.add_to_card(&format!("DCIM/100_FUJI/{name}"), &picture(name));
        }
    }

    pub(crate) fn add_to_card(&self, relative: &str, content: &[u8]) {
        let path = self.card().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// Runs `framehive` on a terminal with the card's mount table, typing `script`.
    pub(crate) fn run(&self, script: &[&str]) -> Transcript {
        finish(self.command(), script)
    }

    /// Like [`Machine::run`], with a stand-in for `udisksctl` first on `PATH` that
    /// records its arguments in [`Machine::eject_arguments`] and then runs `body`.
    pub(crate) fn run_with_udisksctl(&self, body: &str, script: &[&str]) -> Transcript {
        let tools = self.root().join("tools");
        fs::create_dir_all(&tools).unwrap();
        let udisksctl = tools.join("udisksctl");
        let arguments = self.root().join("udisksctl-arguments");
        fs::write(
            &udisksctl,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" > '{}'\n{body}\n",
                arguments.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&udisksctl, fs::Permissions::from_mode(0o755)).unwrap();
        let path = std::env::var("PATH").unwrap_or_default();
        let mut command = self.command();
        command.env("PATH", format!("{}:{path}", tools.display()));
        finish(command, script)
    }

    /// The arguments the stand-in for `udisksctl` was started with, if it was.
    pub(crate) fn eject_arguments(&self) -> String {
        fs::read_to_string(self.root().join("udisksctl-arguments")).unwrap_or_default()
    }

    pub(crate) fn command(&self) -> Command {
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

pub(crate) fn text(path: &Path) -> String {
    path.display().to_string()
}
