// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use files::{Trash, not_a_file};

use crate::ffi;

/// The Trash of macOS, reached through `NSFileManager` like Finder (ADR 0043).
///
/// A file goes to the Trash of its volume, where Finder's "Put Back" restores it.
///
/// ```no_run
/// use std::path::Path;
///
/// use adapter_macos_trash::MacosTrash;
/// use files::Trash;
///
/// let in_trash = MacosTrash::new().move_to_trash(Path::new("/Users/a/DSCF0001.RAF"))?;
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct MacosTrash;

impl MacosTrash {
    /// Creates the adapter.
    #[must_use]
    pub fn new() -> Self {
        MacosTrash
    }
}

impl Trash for MacosTrash {
    fn move_to_trash(&self, path: &Path) -> io::Result<PathBuf> {
        // `trashItemAtURL` would also take a folder or a link, which the port refuses.
        if !fs::symlink_metadata(path)?.is_file() {
            return Err(not_a_file(path));
        }
        ffi::trash_item(&std::path::absolute(path)?)
    }
}
