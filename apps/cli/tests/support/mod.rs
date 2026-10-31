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
        let mut arguments = vec![
            "import".to_owned(),
            self.source().display().to_string(),
            destination.display().to_string(),
        ];
        arguments.extend(options.iter().map(|option| (*option).to_owned()));
        Command::new(env!("CARGO_BIN_EXE_framehive"))
            .args(arguments)
            .env("TZ", "UTC")
            .output()
            .unwrap()
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
