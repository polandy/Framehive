// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! An opened camera shared between threads (IMPORT-015, ADR 0035).

use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use domain::Timestamp;
use files::{EntryKind, FolderEntry, ReadOnlyFileSystem, ReadSeek};

/// An opened camera's file system, shared between the import job that reads it and
/// the dialog that watches whether it still answers (IMPORT-015, CLI-014). A camera
/// must not be used from two threads at once, so each call takes a lock.
#[derive(Clone)]
pub(crate) struct SharedCamera(Arc<Mutex<Box<dyn ReadOnlyFileSystem + Send>>>);

impl SharedCamera {
    pub(crate) fn new(file_system: Box<dyn ReadOnlyFileSystem + Send>) -> Self {
        SharedCamera(Arc::new(Mutex::new(file_system)))
    }

    /// The file system. A call that panicked leaves nothing half done that a read
    /// could see, so a poisoned lock is used as it is.
    fn locked(&self) -> MutexGuard<'_, Box<dyn ReadOnlyFileSystem + Send>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ReadOnlyFileSystem for SharedCamera {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.locked().entry(path)
    }

    fn list_folder(&self, path: &Path) -> io::Result<Vec<FolderEntry>> {
        self.locked().list_folder(path)
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        self.locked().file_size(path)
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        self.locked().modification_time(path)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        self.locked().open_read(path)
    }
}
