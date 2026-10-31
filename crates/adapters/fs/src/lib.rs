// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `fs` adapter of Framehive: it implements the [`files::FileSystem`] port over
//! `std::fs` (ADR 0020 §3, §4). Only apps depend on it. See `README.md`.
//!
//! ```
//! use std::io::Write;
//!
//! use adapter_fs::StdFileSystem;
//! use files::FileSystem;
//! use files::testing::TempDir;
//!
//! let folder = TempDir::new()?;
//! let file_system = StdFileSystem::new();
//! let part = folder.path().join("photo.jpg.part");
//! let target = folder.path().join("photo.jpg");
//! let mut file = file_system.create_file(&part)?;
//! file.write_all(b"jpeg")?;
//! file.sync()?;
//! file_system.rename_no_replace(&part, &target)?;
//! file_system.sync_folder(folder.path())?;
//! assert!(target.is_file());
//! # Ok::<(), std::io::Error>(())
//! ```

#![forbid(unsafe_code)]

mod system_time;

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use domain::Timestamp;
use files::{EntryKind, FileSystem, ReadSeek, WriteFile};

/// Creates a second name for a file, as [`fs::hard_link`] does. A seam, so that tests
/// can stand in for a file system without hard links.
type LinkFn = fn(&Path, &Path) -> io::Result<()>;

/// The [`FileSystem`] of the machine, over `std::fs`.
///
/// Symbolic links are not followed by [`FileSystem::entry`]; they are
/// [`EntryKind::Other`], so a scan cannot loop or leave its source (IMPORT-005).
#[derive(Debug, Clone, Copy)]
pub struct StdFileSystem {
    link: LinkFn,
}

impl StdFileSystem {
    /// Creates the file system adapter.
    #[must_use]
    pub fn new() -> Self {
        StdFileSystem { link: hard_link }
    }

    /// Renames without hard links, where the file system has none (exFAT, some SMB
    /// shares; ADR 0020 §3).
    ///
    /// It checks that nothing exists at `to`, then renames. Another program that
    /// creates `to` between the check and the rename gets its file replaced; that
    /// race is accepted (ADR 0020 §3), because these file systems offer no rename
    /// that refuses to replace.
    fn rename_without_link(from: &Path, to: &Path, link_error: io::Error) -> io::Result<()> {
        match fs::symlink_metadata(from) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => return Err(link_error),
            Err(error) => return Err(error),
        }
        match fs::symlink_metadata(to) {
            Ok(_) => return Err(already_exists(to)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::rename(from, to)
    }
}

impl Default for StdFileSystem {
    fn default() -> Self {
        StdFileSystem::new()
    }
}

impl FileSystem for StdFileSystem {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.is_file() => Ok(Some(EntryKind::File)),
            Ok(metadata) if metadata.is_dir() => Ok(Some(EntryKind::Folder)),
            Ok(_) => Ok(Some(EntryKind::Other)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn create_folder(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        let file = File::open(path)?;
        // Opening a folder succeeds on Unix; only reading it fails.
        if file.metadata()?.is_dir() {
            return Err(is_a_folder(path));
        }
        Ok(Box::new(file))
    }

    fn create_file(&self, path: &Path) -> io::Result<Box<dyn WriteFile>> {
        Ok(Box::new(StdWriteFile(File::create(path)?)))
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        let metadata = fs::metadata(path)?;
        if metadata.is_dir() {
            return Err(is_a_folder(path));
        }
        system_time::to_timestamp(metadata.modified()?)
    }

    /// Links `to` to the file and removes the name `from`, which never replaces: the
    /// link fails if `to` exists. Without hard links it falls back to a checked
    /// rename (see [`StdFileSystem`]'s private `rename_without_link`).
    fn rename_no_replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        match (self.link)(from, to) {
            Ok(()) => remove_old_name(from, to),
            Err(error) if has_no_hard_links(&error) => {
                StdFileSystem::rename_without_link(from, to, error)
            }
            Err(error) => Err(error),
        }
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn sync_folder(&self, path: &Path) -> io::Result<()> {
        File::open(path)?.sync_all()
    }
}

fn hard_link(from: &Path, to: &Path) -> io::Result<()> {
    fs::hard_link(from, to)
}

/// Removes `from` after it was linked to `to`. If that fails, it removes `to` again,
/// so that both names are as before the call.
fn remove_old_name(from: &Path, to: &Path) -> io::Result<()> {
    fs::remove_file(from).inspect_err(|_| {
        // The original error is the one to report; `from` still holds the file.
        let _ = fs::remove_file(to);
    })
}

/// File systems without hard links report `EPERM` (exFAT) or `EOPNOTSUPP`.
fn has_no_hard_links(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::Unsupported | io::ErrorKind::PermissionDenied
    )
}

fn already_exists(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("{} already exists", path.display()),
    )
}

fn is_a_folder(path: &Path) -> io::Error {
    io::Error::other(format!("{} is a folder", path.display()))
}

/// A file opened for writing by [`StdFileSystem::create_file`].
struct StdWriteFile(File);

impl Write for StdWriteFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl WriteFile for StdWriteFile {
    fn set_modification_time(&mut self, time: Timestamp) -> io::Result<()> {
        self.0.set_modified(system_time::to_system_time(time)?)
    }

    fn sync(&mut self) -> io::Result<()> {
        self.0.sync_all()
    }
}

#[cfg(test)]
mod tests;
