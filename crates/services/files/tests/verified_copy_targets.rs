// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The verified copy to several targets (IMPORT-007, ADR 0030 §1), against
//! `MemoryFileSystem`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers may panic on bad fixtures"
)]

use std::cell::Cell;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use domain::{Checksum, Timestamp};
use files::testing::{Fault, FnvHasher, MemoryFileSystem};
use files::{
    CopyErrorKind, CopyStep, EntryKind, FileSystem, FolderEntry, Hasher, ReadOnlyFileSystem,
    ReadSeek, VerifiedCopies, WriteFile, copy_verified_to_all,
};

const SOURCE: &str = "/card/DCIM/100_FUJI/DSCF0001.RAF";
const IMPORT_TARGET: &str = "/photos/2026/DSCF0001.RAF";
const FIRST_BACKUP_TARGET: &str = "/backup-1/2026/DSCF0001.RAF";
const SECOND_BACKUP_TARGET: &str = "/backup-2/2026/DSCF0001.RAF";
/// 2026-10-17 14:05:09 UTC
const MODIFIED_SECONDS: i64 = 1_792_245_909;

/// A source of several copy buffers, so that a target can fail half-way.
fn source_content() -> Vec<u8> {
    (0..=250).cycle().take(200_003).collect()
}

fn with_source() -> MemoryFileSystem {
    let file_system = MemoryFileSystem::new();
    let modified = Timestamp::from_unix_seconds(MODIFIED_SECONDS);
    file_system.add_file(Path::new(SOURCE), &source_content(), modified);
    file_system
}

fn targets() -> [&'static Path; 3] {
    [
        Path::new(IMPORT_TARGET),
        Path::new(FIRST_BACKUP_TARGET),
        Path::new(SECOND_BACKUP_TARGET),
    ]
}

fn copy_to_all(file_system: &dyn FileSystem, targets: &[&Path]) -> VerifiedCopies {
    copy_verified_to_all(
        file_system,
        file_system,
        &FnvHasher,
        Path::new(SOURCE),
        targets,
    )
}

fn checksum_of(content: &[u8]) -> Checksum {
    let mut hash = FnvHasher.start();
    hash.update(content);
    hash.finish()
}

fn part(target: &str) -> PathBuf {
    PathBuf::from(format!("{target}.part"))
}

/// A file system that counts how often the source is opened and how many of its
/// bytes are read.
struct CountingReads<'a> {
    inner: &'a MemoryFileSystem,
    opened: Cell<usize>,
    bytes_read: Rc<Cell<usize>>,
}

struct CountingReader {
    inner: Box<dyn ReadSeek>,
    bytes_read: Rc<Cell<usize>>,
}

impl Read for CountingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buffer)?;
        self.bytes_read.set(self.bytes_read.get() + count);
        Ok(count)
    }
}

impl Seek for CountingReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}

impl ReadOnlyFileSystem for CountingReads<'_> {
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
        let reader = self.inner.open_read(path)?;
        if path != Path::new(SOURCE) {
            return Ok(reader);
        }
        self.opened.set(self.opened.get() + 1);
        Ok(Box::new(CountingReader {
            inner: reader,
            bytes_read: Rc::clone(&self.bytes_read),
        }))
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        self.inner.modification_time(path)
    }
}

impl FileSystem for CountingReads<'_> {
    fn create_folder(&self, path: &Path) -> io::Result<()> {
        self.inner.create_folder(path)
    }

    fn create_file(&self, path: &Path) -> io::Result<Box<dyn WriteFile>> {
        self.inner.create_file(path)
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

/// spec: IMPORT-007.3
#[test]
fn source_is_read_once_for_every_target() {
    let memory = with_source();
    let file_system = CountingReads {
        inner: &memory,
        opened: Cell::new(0),
        bytes_read: Rc::new(Cell::new(0)),
    };

    let copies = copy_to_all(&file_system, &targets());

    assert_eq!(file_system.opened.get(), 1);
    assert_eq!(file_system.bytes_read.get(), source_content().len());
    assert!(copies.targets.iter().all(Result::is_ok));
}

/// A source that is another storage device than the targets, such as a camera, is
/// only read (ADR 0034).
#[test]
fn source_on_its_own_file_system_is_only_read() {
    let source = with_source();
    let destination = MemoryFileSystem::new();

    let copies = copy_verified_to_all(
        &source,
        &destination,
        &FnvHasher,
        Path::new(SOURCE),
        &targets(),
    );

    assert!(copies.targets.iter().all(Result::is_ok));
    for target in targets() {
        assert_eq!(destination.content(target), Some(source_content()));
    }
    assert_eq!(source.paths(), with_source().paths());
}

/// spec: IMPORT-007.1
#[test]
fn every_target_gets_the_content_and_modification_time() {
    let file_system = with_source();

    let copies = copy_to_all(&file_system, &targets());

    let checksum = checksum_of(&source_content());
    assert_eq!(copies.source_checksum.as_ref(), Some(&checksum));
    for (target, result) in targets().into_iter().zip(&copies.targets) {
        assert_eq!(result.as_ref().unwrap(), &checksum);
        assert_eq!(file_system.content(target), Some(source_content()));
        assert_eq!(
            file_system.modified(target),
            Some(Timestamp::from_unix_seconds(MODIFIED_SECONDS))
        );
    }
}

/// spec: IMPORT-007.5
#[test]
fn a_failed_part_file_drops_only_its_target() {
    let file_system = with_source();
    let failing_part = part(FIRST_BACKUP_TARGET);
    file_system.inject_fault(
        &failing_part,
        Fault::WriteFails {
            after_bytes: 70_000,
        },
    );

    let copies = copy_to_all(&file_system, &targets());

    let [import, first_backup, second_backup] = &copies.targets[..] else {
        panic!("one result per target");
    };
    assert!(import.is_ok());
    assert!(second_backup.is_ok());
    assert_eq!(first_backup.as_ref().unwrap_err().step, CopyStep::WritePart);
    assert_eq!(file_system.entry(&failing_part).unwrap(), None);
    assert_eq!(
        file_system.entry(Path::new(FIRST_BACKUP_TARGET)).unwrap(),
        None
    );
    assert_eq!(
        file_system.content(Path::new(SECOND_BACKUP_TARGET)),
        Some(source_content())
    );
}

/// spec: IMPORT-007.6
#[test]
fn a_failed_first_target_keeps_the_others() {
    let file_system = with_source();
    file_system.inject_fault(&part(IMPORT_TARGET), Fault::WriteFails { after_bytes: 0 });

    let copies = copy_to_all(&file_system, &targets());

    assert_eq!(
        copies.targets[0].as_ref().unwrap_err().step,
        CopyStep::WritePart
    );
    assert!(copies.targets[1..].iter().all(Result::is_ok));
    assert_eq!(file_system.entry(Path::new(IMPORT_TARGET)).unwrap(), None);
    assert_eq!(file_system.entry(&part(IMPORT_TARGET)).unwrap(), None);
}

/// spec: IMPORT-007.4
#[test]
fn each_target_is_verified_on_its_own() {
    let file_system = with_source();
    file_system.inject_fault(
        Path::new(SECOND_BACKUP_TARGET),
        Fault::ReadReturnsDifferentBytes,
    );

    let copies = copy_to_all(&file_system, &targets());

    assert!(copies.targets[..2].iter().all(Result::is_ok));
    let error = copies.targets[2].as_ref().unwrap_err();
    assert_eq!(error.step, CopyStep::Verify);
    assert!(matches!(error.kind, CopyErrorKind::ChecksumMismatch { .. }));
    assert_eq!(
        file_system.entry(Path::new(SECOND_BACKUP_TARGET)).unwrap(),
        None
    );
}

#[test]
fn the_read_ends_early_when_every_target_failed() {
    let memory = with_source();
    for target in targets() {
        memory.inject_fault(
            &part(target.to_str().unwrap()),
            Fault::WriteFails { after_bytes: 0 },
        );
    }
    let file_system = CountingReads {
        inner: &memory,
        opened: Cell::new(0),
        bytes_read: Rc::new(Cell::new(0)),
    };

    let copies = copy_to_all(&file_system, &targets());

    assert_eq!(copies.source_checksum, None);
    assert!(copies.targets.iter().all(Result::is_err));
    assert!(file_system.bytes_read.get() < source_content().len());
}

#[test]
fn a_source_that_cannot_be_read_fails_every_target() {
    let file_system = with_source();
    file_system.inject_fault(
        Path::new(SOURCE),
        Fault::ReadFails {
            after_bytes: 70_000,
        },
    );

    let copies = copy_to_all(&file_system, &targets());

    for (target, result) in targets().into_iter().zip(&copies.targets) {
        assert_eq!(result.as_ref().unwrap_err().step, CopyStep::ReadSource);
        assert_eq!(file_system.entry(target).unwrap(), None);
        assert_eq!(
            file_system.entry(&part(target.to_str().unwrap())).unwrap(),
            None
        );
    }
}

#[test]
fn no_targets_read_nothing() {
    let memory = with_source();
    let file_system = CountingReads {
        inner: &memory,
        opened: Cell::new(0),
        bytes_read: Rc::new(Cell::new(0)),
    };

    let copies = copy_to_all(&file_system, &[]);

    assert!(copies.targets.is_empty());
    assert_eq!(file_system.opened.get(), 0);
}
