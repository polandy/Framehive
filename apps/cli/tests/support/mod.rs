// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Folders, test files and output helpers shared by the tests of `framehive import`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]
#![allow(dead_code, reason = "each test binary uses only some of the helpers")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime};

use files::testing::TempDir;
use metadata::testing::{ByteOrder, DATE_TIME_ORIGINAL, Tiff, jpeg, raf};

/// 2026-10-17 14:05:09 UTC.
pub(crate) const MODIFIED: u64 = 1_792_245_909;
pub(crate) const CAPTURE_TIME: &str = "2026:10:17 14:05:09";
pub(crate) const DEFAULT_TARGET: &str = "2026/2026-10-17/DSCF0001.RAF";
pub(crate) const COPIED_LINE: &str = "copied  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF";

/// A temporary folder with a source folder and the path of a destination that does
/// not exist yet.
pub(crate) struct Folders {
    pub(crate) root: TempDir,
}

impl Folders {
    pub(crate) fn new() -> Self {
        let folders = Folders {
            root: TempDir::new().unwrap(),
        };
        fs::create_dir(folders.source()).unwrap();
        folders
    }

    pub(crate) fn source(&self) -> PathBuf {
        self.root.path().join("source")
    }

    pub(crate) fn destination(&self) -> PathBuf {
        self.root.path().join("library")
    }

    /// The counter file of the persisted sequence counters (ADR 0029). Every run gets
    /// its state folder below the root, so no test touches the real home folder.
    pub(crate) fn counter_file(&self) -> PathBuf {
        self.root
            .path()
            .join("state/framehive/sequence-counters.txt")
    }

    /// The import history of the state folder (CLI-008).
    pub(crate) fn history_file(&self) -> PathBuf {
        self.root
            .path()
            .join("state/framehive/import-history.sqlite")
    }

    /// Deletes the import history, with the journal files SQLite may leave, so a
    /// later run knows no earlier import, as on another machine.
    pub(crate) fn forget_history(&self) {
        let history = self.history_file();
        for suffix in ["", "-wal", "-shm"] {
            let mut file = history.clone().into_os_string();
            file.push(suffix);
            if Path::new(&file).exists() {
                fs::remove_file(&file).unwrap();
            }
        }
    }

    /// Writes `text` as the counter file.
    pub(crate) fn write_counters(&self, text: &str) {
        let file = self.counter_file();
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }

    /// Adds a file to the source, modified at [`MODIFIED`].
    pub(crate) fn add(&self, relative: &str, content: &[u8]) -> PathBuf {
        let path = self.source().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        let file = fs::File::options().write(true).open(&path).unwrap();
        file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(MODIFIED))
            .unwrap();
        path
    }

    pub(crate) fn add_raf(&self) {
        self.add("DSCF0001.RAF", &raf(&photo()));
    }

    pub(crate) fn import(&self, options: &[&str]) -> Output {
        self.import_into(&self.destination(), options)
    }

    pub(crate) fn import_into(&self, destination: &Path, options: &[&str]) -> Output {
        self.import_command(destination, options)
            .env("XDG_STATE_HOME", self.root.path().join("state"))
            .output()
            .unwrap()
    }

    /// Runs the import of the source with `options`, with neither `XDG_STATE_HOME` nor
    /// an absolute `HOME`, so no state folder can be found.
    pub(crate) fn import_without_state_folder(&self, options: &[&str]) -> Output {
        self.import_command(&self.destination(), options)
            .env_remove("XDG_STATE_HOME")
            .env("HOME", "not-absolute")
            .output()
            .unwrap()
    }

    /// Runs `framehive import-log` with `arguments` on the state folder of the imports
    /// (CLI-009).
    pub(crate) fn import_log(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_framehive"))
            .arg("import-log")
            .args(arguments)
            .env("TZ", "UTC")
            .env("XDG_STATE_HOME", self.root.path().join("state"))
            .output()
            .unwrap()
    }

    /// The home folder of the profile runs (CLI-010), below the root, so no test reads
    /// the real home folder.
    pub(crate) fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    /// The configuration folder of CLI-010 for [`Folders::home`] on this platform, with
    /// `XDG_CONFIG_HOME` not set.
    pub(crate) fn configuration_folder(&self) -> PathBuf {
        if cfg!(target_os = "macos") {
            self.home().join("Library/Application Support/framehive")
        } else {
            self.home().join(".config/framehive")
        }
    }

    pub(crate) fn profiles_folder(&self) -> PathBuf {
        self.configuration_folder().join("profiles")
    }

    /// Writes `text` as the profile `<name>.yaml` and returns its path.
    pub(crate) fn write_profile(&self, name: &str, text: &[u8]) -> PathBuf {
        let path = self.profiles_folder().join(format!("{name}.yaml"));
        fs::create_dir_all(self.profiles_folder()).unwrap();
        fs::write(&path, text).unwrap();
        path
    }

    /// Runs `framehive` with `arguments`, [`Folders::home`] as `HOME`, no
    /// `XDG_CONFIG_HOME` and the state folder below the root.
    pub(crate) fn framehive(&self, arguments: &[&str]) -> Output {
        self.framehive_command(arguments).output().unwrap()
    }

    /// The command of [`Folders::framehive`], for a test that changes its environment.
    pub(crate) fn framehive_command(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_framehive"));
        command
            .args(arguments)
            .env("TZ", "UTC")
            .env("HOME", self.home())
            .env_remove("XDG_CONFIG_HOME")
            .env("XDG_STATE_HOME", self.root.path().join("state"));
        command
    }

    fn import_command(&self, destination: &Path, options: &[&str]) -> Command {
        let mut arguments = vec![
            "import".to_owned(),
            self.source().display().to_string(),
            destination.display().to_string(),
        ];
        arguments.extend(options.iter().map(|option| (*option).to_owned()));
        let mut command = Command::new(env!("CARGO_BIN_EXE_framehive"));
        command.args(arguments).env("TZ", "UTC");
        command
    }
}

pub(crate) fn photo() -> Vec<u8> {
    jpeg(
        &Tiff::new(ByteOrder::LittleEndian)
            .with_exif_text(DATE_TIME_ORIGINAL, CAPTURE_TIME)
            .to_bytes(),
    )
}

pub(crate) fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub(crate) fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub(crate) fn lines(output: &Output) -> Vec<String> {
    stdout(output).lines().map(str::to_owned).collect()
}

/// The summary line of a run (CLI-001); the format readiness (CLI-007) follows it.
pub(crate) fn summary_line(output: &Output) -> String {
    lines(output)
        .into_iter()
        .find(|line| line.contains(" already imported, "))
        .unwrap_or_default()
}

pub(crate) fn has_line(output: &Output, line: &str) -> bool {
    lines(output).iter().any(|printed| printed == line)
}

pub(crate) fn has_line_starting_with(output: &Output, start: &str) -> bool {
    lines(output)
        .iter()
        .any(|printed| printed.starts_with(start))
}

pub(crate) fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}
