// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The read-back of the verified copy comes from the storage device (IMPORT-004.15,
//! ADR 0047), against `MemoryFileSystem` behind a device that stored other bytes than
//! its cache holds.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers may panic on bad fixtures"
)]

use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};

use domain::{Checksum, Timestamp};
use files::testing::{FnvHasher, MemoryFileSystem};
use files::{
    CopyErrorKind, CopyStep, EntryKind, FileSystem, FolderEntry, Hasher, ReadOnlyFileSystem,
    ReadSeek, WriteFile, copy_verified,
};

const SOURCE: &str = "/card/DSCF0001.RAF";
const TARGET: &str = "/photos/DSCF0001.RAF";
const CONTENT: &[u8] = b"raw content";

/// A file system whose `open_read` answers from the cache, which holds what was
/// written, while the device holds the target with its last byte flipped.
struct CorruptingDevice<'a> {
    inner: &'a MemoryFileSystem,
}

impl ReadOnlyFileSystem for CorruptingDevice<'_> {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.inner.entry(path)
    }

    fn list_folder(&self, path: &Path) -> io::Result<Vec<FolderEntry>> {
        self.inner.list_folder(path)
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        self.inner.file_size(path)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        self.inner.open_read(path)
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        self.inner.modification_time(path)
    }
}

impl FileSystem for CorruptingDevice<'_> {
    fn create_folder(&self, path: &Path) -> io::Result<()> {
        self.inner.create_folder(path)
    }

    fn create_file(&self, path: &Path) -> io::Result<Box<dyn WriteFile>> {
        self.inner.create_file(path)
    }

    fn rename_no_replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename_no_replace(from, to)
    }

    fn replace_file(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.replace_file(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_file(path)
    }

    fn sync_folder(&self, path: &Path) -> io::Result<()> {
        self.inner.sync_folder(path)
    }

    fn open_read_from_medium(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        let mut stored = Vec::new();
        self.inner.open_read(path)?.read_to_end(&mut stored)?;
        if path == Path::new(TARGET) {
            stored = corrupted(CONTENT);
        }
        Ok(Box::new(Cursor::new(stored)))
    }
}

fn corrupted(content: &[u8]) -> Vec<u8> {
    let mut stored = content.to_vec();
    if let Some(last) = stored.last_mut() {
        *last = !*last;
    }
    stored
}

fn checksum_of(content: &[u8]) -> Checksum {
    let mut hash = FnvHasher.start();
    hash.update(content);
    hash.finish()
}

/// spec: IMPORT-004.15
#[test]
fn a_device_that_stored_other_bytes_than_its_cache_fails_verification() {
    let memory = MemoryFileSystem::new();
    memory.add_file(Path::new(SOURCE), CONTENT, Timestamp::UNIX_EPOCH);
    memory.add_folder(Path::new("/photos"));
    let target = PathBuf::from(TARGET);

    let error = copy_verified(
        &CorruptingDevice { inner: &memory },
        &FnvHasher,
        Path::new(SOURCE),
        &target,
    )
    .unwrap_err();

    assert_eq!(error.step, CopyStep::Verify);
    let CopyErrorKind::ChecksumMismatch {
        source_checksum,
        target_checksum,
    } = &error.kind
    else {
        panic!("expected a checksum mismatch, got {error}");
    };
    assert_eq!(source_checksum, &checksum_of(CONTENT));
    assert_eq!(target_checksum, &checksum_of(&corrupted(CONTENT)));
    assert_eq!(memory.entry(&target).unwrap(), None);
    assert_eq!(
        memory
            .entry(Path::new("/photos/DSCF0001.RAF.part"))
            .unwrap(),
        None
    );
}
