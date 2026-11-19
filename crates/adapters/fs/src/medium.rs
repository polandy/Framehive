// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading a file from the storage device instead of the page cache (ADR 0047).

use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;

/// Drops the cached pages of `file`, so that the next read comes from the device.
/// Only clean pages are dropped; a synced file has no other.
#[cfg(not(target_os = "macos"))]
pub(crate) fn bypass_cache(file: &File) -> io::Result<()> {
    // SAFETY: the descriptor belongs to `file`, which stays open during the call, and
    // the call takes no pointer.
    let result = unsafe { libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) };
    if result != 0 {
        return Err(io::Error::from_raw_os_error(result));
    }
    Ok(())
}

/// Turns caching off for reads of `file`. macOS has no call that drops pages already
/// cached, so this is best effort (ADR 0047 §2).
#[cfg(target_os = "macos")]
pub(crate) fn bypass_cache(file: &File) -> io::Result<()> {
    // SAFETY: the descriptor belongs to `file`, which stays open during the call, and
    // `F_NOCACHE` takes an integer, no pointer.
    let result = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_NOCACHE, 1) };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
