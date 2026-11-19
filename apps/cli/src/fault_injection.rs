// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The fault-injection point of the binary (ADR 0048): a [`FileSystem`] that wraps the
//! one of the import, compiled only with the cargo feature `fault-injection`.

use std::io;
use std::path::Path;

use domain::Timestamp;
use files::{EntryKind, FileSystem, FolderEntry, ReadOnlyFileSystem, ReadSeek, WriteFile};

/// A [`FileSystem`] that passes every call through to the one it wraps.
pub(crate) struct FaultyFileSystem<F> {
    inner: F,
}

impl<F: FileSystem> FaultyFileSystem<F> {
    /// Wraps `inner`.
    pub(crate) fn new(inner: F) -> Self {
        FaultyFileSystem { inner }
    }
}

impl<F: FileSystem> ReadOnlyFileSystem for FaultyFileSystem<F> {
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

impl<F: FileSystem> FileSystem for FaultyFileSystem<F> {
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
        self.inner.open_read_from_medium(path)
    }
}

#[cfg(test)]
mod tests {
    use adapter_fs::StdFileSystem;
    use files::testing::{FileSystemUnderTest, TempDir, check_file_system};

    use super::FaultyFileSystem;

    /// Without a fault, the decorator conforms to the port as the file system it wraps.
    /// spec: IMPORT-004.14
    #[test]
    fn conforms_to_the_file_system_port_without_a_fault() {
        let folder = TempDir::new().unwrap();
        let failures = check_file_system(&FileSystemUnderTest {
            file_system: &FaultyFileSystem::new(StdFileSystem::new()),
            root: folder.path(),
            crash: None,
            make_symbolic_link: Some(&|link, target| std::os::unix::fs::symlink(target, link)),
        });
        assert!(failures.is_empty(), "{failures:#?}");
    }
}
