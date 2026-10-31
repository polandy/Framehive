// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A temporary folder for tests that touch a real file system (ADR 0020 §4).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

/// How many names are tried before giving up; each try uses a new number, so a
/// clash means another program uses the same naming scheme.
const MAX_ATTEMPTS: u32 = 1_000;

static NEXT_NUMBER: AtomicU64 = AtomicU64::new(0);

/// A new, empty folder in the system's temporary folder, removed with everything in
/// it when dropped.
///
/// Its name is made of the process id and a counter, so it needs no randomness.
///
/// ```
/// use files::testing::TempDir;
///
/// let folder = TempDir::new()?;
/// let path = folder.path().to_path_buf();
/// assert!(path.is_dir());
/// drop(folder);
/// assert!(!path.exists());
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates the folder.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder cannot be created.
    pub fn new() -> io::Result<Self> {
        let base = std::env::temp_dir();
        for _ in 0..MAX_ATTEMPTS {
            let number = NEXT_NUMBER.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!("framehive-test-{}-{number}", process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(TempDir { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("no free name for a temporary folder in {}", base.display()),
        ))
    }

    /// Returns the path of the folder.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Nothing can be reported from a drop; a folder left behind is only clutter.
        let _ = fs::remove_dir_all(&self.path);
    }
}
