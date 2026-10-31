// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A file system that counts what the capture time reader opens and reads.

use std::cell::Cell;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::rc::Rc;

use domain::Timestamp;
use files::testing::MemoryFileSystem;
use files::{EntryKind, FileSystem, ReadSeek, WriteFile};

/// `MemoryFileSystem` that counts opened files and the bytes read from them.
pub(crate) struct CountingFileSystem {
    inner: MemoryFileSystem,
    opened: Cell<usize>,
    bytes_read: Rc<Cell<u64>>,
}

impl CountingFileSystem {
    /// An empty file system that has counted nothing.
    pub(crate) fn new() -> Self {
        CountingFileSystem {
            inner: MemoryFileSystem::new(),
            opened: Cell::new(0),
            bytes_read: Rc::new(Cell::new(0)),
        }
    }

    /// Adds a file with `content`.
    pub(crate) fn add_file(&self, path: &Path, content: &[u8]) {
        self.inner.add_file(path, content, Timestamp::UNIX_EPOCH);
    }

    /// How many files were opened for reading.
    pub(crate) fn opened(&self) -> usize {
        self.opened.get()
    }

    /// How many bytes were read from all opened files.
    pub(crate) fn bytes_read(&self) -> u64 {
        self.bytes_read.get()
    }
}

impl FileSystem for CountingFileSystem {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.inner.entry(path)
    }
    fn create_folder(&self, path: &Path) -> io::Result<()> {
        self.inner.create_folder(path)
    }
    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        self.opened.set(self.opened.get() + 1);
        Ok(Box::new(CountingReader {
            inner: self.inner.open_read(path)?,
            bytes_read: Rc::clone(&self.bytes_read),
        }))
    }
    fn create_file(&self, path: &Path) -> io::Result<Box<dyn WriteFile>> {
        self.inner.create_file(path)
    }
    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        self.inner.modification_time(path)
    }
    fn rename_no_replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename_no_replace(from, to)
    }
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_file(path)
    }
    fn sync_folder(&self, path: &Path) -> io::Result<()> {
        self.inner.sync_folder(path)
    }
}

struct CountingReader {
    inner: Box<dyn ReadSeek>,
    bytes_read: Rc<Cell<u64>>,
}

impl Read for CountingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buffer)?;
        let count_u64 = u64::try_from(count).unwrap_or(u64::MAX);
        self.bytes_read.set(self.bytes_read.get() + count_u64);
        Ok(count)
    }
}

impl Seek for CountingReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}
