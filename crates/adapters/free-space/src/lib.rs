// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `free-space` adapter of Framehive: it implements the [`FreeSpace`] port of
//! `files` with `statvfs(3)` (ADR 0033). Only apps depend on it. See `README.md`.

use std::ffi::CString;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use files::{FileSystemId, FreeSpace, SpaceOnFileSystem};

/// The free space of the file systems of this machine, read with `statvfs(3)` (ADR
/// 0033).
///
/// The free space is `f_bavail × f_frsize`, the bytes an unprivileged process may
/// still write; the identity of the file system is `st_dev` of the folder.
///
/// ```
/// use files::FreeSpace;
/// use adapter_free_space::StatvfsFreeSpace;
///
/// let space = StatvfsFreeSpace::new().free_space(&std::env::temp_dir())?;
/// let _bytes_free: u64 = space.available_bytes;
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct StatvfsFreeSpace;

impl StatvfsFreeSpace {
    /// Creates the adapter.
    #[must_use]
    pub fn new() -> Self {
        StatvfsFreeSpace
    }
}

impl FreeSpace for StatvfsFreeSpace {
    fn free_space(&self, folder: &Path) -> io::Result<SpaceOnFileSystem> {
        let existing = nearest_existing(folder)?;
        let file_system = FileSystemId::new(fs::metadata(existing)?.dev());
        let available_bytes = available_bytes(existing)?;
        Ok(SpaceOnFileSystem {
            file_system,
            available_bytes,
        })
    }
}

/// `path` itself, or its nearest parent that exists. A path that cannot be inspected
/// for another reason than not existing is an error, so a folder the user may not
/// read is not answered for the file system of its parent.
fn nearest_existing(path: &Path) -> io::Result<&Path> {
    for candidate in path.ancestors() {
        match fs::metadata(candidate) {
            Ok(_) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("no parent folder of {} exists", path.display()),
    ))
}

/// The bytes an unprivileged process may still write on the file system of the
/// existing `path`.
fn available_bytes(path: &Path) -> io::Result<u64> {
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let mut stats = libc::statvfs::default();
    // SAFETY: `path` is a NUL-terminated string that lives until the call returns,
    // and `stats` is a valid, writable `statvfs` the call fills in; it keeps neither
    // pointer.
    let result = unsafe { libc::statvfs(path.as_ptr(), &raw mut stats) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    #[allow(
        clippy::useless_conversion,
        reason = "the fields are u64 on Linux, but f_bavail is u32 on macOS"
    )]
    let (blocks, block_size) = (u64::from(stats.f_bavail), u64::from(stats.f_frsize));
    blocks.checked_mul(block_size).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "the free space does not fit in 64 bits",
        )
    })
}
