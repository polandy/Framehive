// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The checksum of a file's content.

use std::io::{self, Read};
use std::path::Path;

use domain::Checksum;

use crate::{FileSystem, Hasher};

/// How many bytes are read and hashed at a time.
pub(crate) const BUFFER_SIZE: usize = 64 * 1024;

/// Reads the file `path` in full and returns the checksum of its content.
///
/// The import uses it to tell whether a file at a target path holds the same content
/// as the source (IMPORT-003); the verified copy uses it to read the target back
/// (IMPORT-004).
///
/// ```
/// use std::path::Path;
///
/// use domain::Timestamp;
/// use files::checksum_file;
/// use files::testing::{FnvHasher, MemoryFileSystem};
///
/// let file_system = MemoryFileSystem::new();
/// file_system.add_file(Path::new("/card/a.jpg"), b"jpeg", Timestamp::UNIX_EPOCH);
/// file_system.add_file(Path::new("/card/b.jpg"), b"jpeg", Timestamp::UNIX_EPOCH);
///
/// let a = checksum_file(&file_system, &FnvHasher, Path::new("/card/a.jpg"))?;
/// let b = checksum_file(&file_system, &FnvHasher, Path::new("/card/b.jpg"))?;
/// assert_eq!(a, b);
/// # Ok::<(), std::io::Error>(())
/// ```
///
/// # Errors
///
/// Returns an error if the file cannot be opened or read to its end.
pub fn checksum_file(
    file_system: &dyn FileSystem,
    hasher: &dyn Hasher,
    path: &Path,
) -> io::Result<Checksum> {
    let mut reader = file_system.open_read(path)?;
    let mut hash = hasher.start();
    let mut buffer = vec![0; BUFFER_SIZE];
    loop {
        let count = read_some(&mut *reader, &mut buffer)?;
        if count == 0 {
            return Ok(hash.finish());
        }
        hash.update(&buffer[..count]);
    }
}

/// Reads into `buffer`, retrying reads that a signal interrupted.
pub(crate) fn read_some(reader: &mut dyn Read, buffer: &mut [u8]) -> io::Result<usize> {
    loop {
        match reader.read(buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            result => return result,
        }
    }
}
