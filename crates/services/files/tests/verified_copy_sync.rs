// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The verified copy when the target folder cannot be synced (IMPORT-004), against
//! `MemoryFileSystem` behind a file system that fails `sync_folder`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::{Path, PathBuf};

use domain::Timestamp;
use files::testing::{FnvHasher, MemoryFileSystem};
use files::{CopyStep, EntryKind, FileSystem, copy_verified};

/// A file system whose `sync_folder` fails for one folder.
struct FailingFolderSync<'a> {
    inner: &'a MemoryFileSystem,
    folder: PathBuf,
}

impl FileSystem for FailingFolderSync<'_> {
    fn entry(&self, path: &Path) -> std::io::Result<Option<EntryKind>> {
        self.inner.entry(path)
    }
    fn create_folder(&self, path: &Path) -> std::io::Result<()> {
        self.inner.create_folder(path)
    }
    fn open_read(&self, path: &Path) -> std::io::Result<Box<dyn files::ReadSeek>> {
        self.inner.open_read(path)
    }
    fn create_file(&self, path: &Path) -> std::io::Result<Box<dyn files::WriteFile>> {
        self.inner.create_file(path)
    }
    fn modification_time(&self, path: &Path) -> std::io::Result<Timestamp> {
        self.inner.modification_time(path)
    }
    fn rename_no_replace(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        self.inner.rename_no_replace(from, to)
    }
    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        self.inner.remove_file(path)
    }
    fn sync_folder(&self, path: &Path) -> std::io::Result<()> {
        if path == self.folder {
            return Err(std::io::Error::other("folder sync failed"));
        }
        self.inner.sync_folder(path)
    }
}

/// spec: IMPORT-004.6
#[test]
fn target_is_removed_when_the_folder_cannot_be_synced() {
    let memory = MemoryFileSystem::new();
    memory.add_file(Path::new("/card/a.RAF"), b"raw", Timestamp::UNIX_EPOCH);
    memory.add_folder(Path::new("/photos"));
    let target = PathBuf::from("/photos/a.RAF");
    let file_system = FailingFolderSync {
        inner: &memory,
        folder: PathBuf::from("/photos"),
    };

    let error =
        copy_verified(&file_system, &FnvHasher, Path::new("/card/a.RAF"), &target).unwrap_err();

    assert_eq!(error.step, CopyStep::SyncFolder);
    assert_eq!(memory.entry(&target).unwrap(), None);
    assert_eq!(memory.entry(Path::new("/photos/a.RAF.part")).unwrap(), None);
}
