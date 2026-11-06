// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A temporary folder served as a camera by libgphoto2's `directory` driver.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use domain::Timestamp;
use files::testing::TempDir;

/// A temporary folder as a camera; paths on the camera start at the folder.
pub(crate) struct FolderCamera {
    folder: TempDir,
}

impl FolderCamera {
    pub(crate) fn new() -> Self {
        Self {
            folder: TempDir::new().unwrap(),
        }
    }

    /// The port that opens the folder as a camera.
    pub(crate) fn port(&self) -> String {
        format!("disk:{}", self.folder.path().display())
    }

    /// Where the camera path `path` lies in the folder.
    pub(crate) fn local(&self, path: &Path) -> PathBuf {
        self.folder
            .path()
            .join(path.strip_prefix("/").unwrap_or(path))
    }

    pub(crate) fn add_folder(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(self.local(path))
    }

    pub(crate) fn add_file(
        &self,
        path: &Path,
        content: &[u8],
        modified: Timestamp,
    ) -> io::Result<()> {
        let local = self.local(path);
        fs::write(&local, content)?;
        let seconds = u64::try_from(modified.unix_seconds()).map_err(io::Error::other)?;
        File::options()
            .write(true)
            .open(&local)?
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))
    }
}
