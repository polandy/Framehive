// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io;
use std::path::{Path, PathBuf};

/// Port for moving files to the trash of the desktop, where the person can restore
/// them (ADR 0043).
///
/// `adapter-freedesktop-trash` implements it on Linux, `adapter-macos-trash` on macOS;
/// tests use [`testing::MemoryTrash`](crate::testing::MemoryTrash). Every
/// implementation must pass [`testing::check_trash`](crate::testing::check_trash).
pub trait Trash {
    /// Moves the file at `path` to the trash and returns where it lies there. A file
    /// of the same name already in the trash stays; the new one gets a name of its
    /// own. On an error the file stays where it was.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if nothing is at `path`,
    /// [`io::ErrorKind::InvalidInput`] if `path` is a folder, a symbolic link or
    /// anything else that is not a file, and another error if the trash cannot take
    /// the file.
    fn move_to_trash(&self, path: &Path) -> io::Result<PathBuf>;
}

/// The error for a `path` that is not a file, which [`Trash::move_to_trash`] refuses.
#[must_use]
pub fn not_a_file(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{} is not a file", path.display()),
    )
}
