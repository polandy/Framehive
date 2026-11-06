// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A [`FreeSpace`] with fixed answers, for tests.

use std::io;
use std::path::{Path, PathBuf};

use crate::{FileSystemId, FreeSpace, SpaceOnFileSystem};

/// A [`FreeSpace`] with fixed answers, for tests (ADR 0033).
///
/// Each file system is given by the folder it is mounted at. A folder is answered by
/// the file system mounted at its nearest parent folder, whether the folder exists or
/// not, as the real port answers a missing folder by its nearest existing parent. A
/// folder below no file system is not found.
///
/// ```
/// use std::path::Path;
///
/// use files::FreeSpace;
/// use files::testing::MemoryFreeSpace;
///
/// let free_space = MemoryFreeSpace::new()
///     .with_file_system("/", 1_000)
///     .with_file_system("/media/disk", 500);
///
/// let library = free_space.free_space(Path::new("/media/disk/library"))?;
/// let home = free_space.free_space(Path::new("/home"))?;
/// assert_eq!(library.available_bytes, 500);
/// assert_ne!(library.file_system, home.file_system);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct MemoryFreeSpace {
    mounts: Vec<Mount>,
}

#[derive(Debug, Clone)]
struct Mount {
    folder: PathBuf,
    answer: Result<SpaceOnFileSystem, String>,
}

impl MemoryFreeSpace {
    /// Creates a free space without any file system.
    #[must_use]
    pub fn new() -> Self {
        MemoryFreeSpace::default()
    }

    /// One file system at `/` with more space free than any test writes.
    #[must_use]
    pub fn unlimited() -> Self {
        MemoryFreeSpace::new().with_file_system("/", u64::MAX)
    }

    /// Adds a file system mounted at `folder` with `available_bytes` free. Each file
    /// system gets its own identity.
    #[must_use]
    pub fn with_file_system(mut self, folder: impl Into<PathBuf>, available_bytes: u64) -> Self {
        let file_system = FileSystemId::new(self.mounts.len() as u64 + 1);
        self.mounts.push(Mount {
            folder: folder.into(),
            answer: Ok(SpaceOnFileSystem {
                file_system,
                available_bytes,
            }),
        });
        self
    }

    /// Adds a file system mounted at `folder` that does not report its free space:
    /// every answer for it is an error with `reason`.
    #[must_use]
    pub fn with_unreadable(mut self, folder: impl Into<PathBuf>, reason: &str) -> Self {
        self.mounts.push(Mount {
            folder: folder.into(),
            answer: Err(reason.to_owned()),
        });
        self
    }
}

impl FreeSpace for MemoryFreeSpace {
    fn free_space(&self, folder: &Path) -> io::Result<SpaceOnFileSystem> {
        let mount = self
            .mounts
            .iter()
            .filter(|mount| folder.starts_with(&mount.folder))
            .max_by_key(|mount| mount.folder.components().count())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("no file system holds {}", folder.display()),
                )
            })?;
        mount
            .answer
            .clone()
            .map_err(|reason| io::Error::new(io::ErrorKind::Unsupported, reason))
    }
}
