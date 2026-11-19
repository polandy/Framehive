// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The stall limit of a source (IMPORT-022, ADR 0049): each source of a run is read
//! through a [`StallGuard`] of its own.

use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use domain::Timestamp;
use files::{EntryKind, FileSystem, FolderEntry, ReadOnlyFileSystem, ReadSeek, StallGuard};

use crate::job::{ImportPorts, OwnSource};

/// The stall limit of a source when the profile sets none (IMPORT-022).
pub const DEFAULT_SOURCE_STALL_LIMIT: Duration = Duration::from_secs(30);

impl ImportPorts {
    /// A guard around `own`, or around the file system of the destinations for a
    /// source without one of its own, whose every call must answer within `limit` as
    /// the timer of these ports measures it.
    ///
    /// # Errors
    ///
    /// Returns an error if the thread that reads the source cannot be started.
    pub fn guard_source(&self, own: Option<OwnSource>, limit: Duration) -> io::Result<SourceGuard> {
        let source = own.unwrap_or_else(|| Box::new(SharedSource(Arc::clone(&self.file_system))));
        let lent = Arc::new(Mutex::new(Some(source)));
        let guard = StallGuard::new(
            Box::new(LentSource(Arc::clone(&lent))),
            Arc::clone(&self.timer),
            limit,
        )?;
        Ok(SourceGuard { guard, lent })
    }
}

/// A source read through a [`StallGuard`]. The guard only borrows the source, so
/// dropping this guard, or [`SourceGuard::give_back`], releases it at once, such as a
/// camera, unless a call to it did not answer.
pub struct SourceGuard {
    guard: StallGuard,
    lent: Arc<Mutex<Option<OwnSource>>>,
}

impl SourceGuard {
    /// The source, or this guard if a call to the source did not answer, as the
    /// source then stays with the call that waits for it.
    #[must_use]
    pub fn give_back(self) -> OwnSource {
        match self.take_back() {
            Some(source) => source,
            None => Box::new(self),
        }
    }

    fn take_back(&self) -> Option<OwnSource> {
        if self.guard.is_stopped() {
            return None;
        }
        // Free unless a call is under way, and none is once the guard is not borrowed.
        self.lent.try_lock().ok()?.take()
    }
}

impl Drop for SourceGuard {
    fn drop(&mut self) {
        drop(self.take_back());
    }
}

impl ReadOnlyFileSystem for SourceGuard {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.guard.entry(path)
    }

    fn list_folder(&self, path: &Path) -> io::Result<Vec<FolderEntry>> {
        self.guard.list_folder(path)
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        self.guard.file_size(path)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        self.guard.open_read(path)
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        self.guard.modification_time(path)
    }
}

/// The source as the source thread reads it, until the [`SourceGuard`] takes it back.
struct LentSource(Arc<Mutex<Option<OwnSource>>>);

impl LentSource {
    fn with<T>(
        &self,
        call: impl FnOnce(&dyn ReadOnlyFileSystem) -> io::Result<T>,
    ) -> io::Result<T> {
        // A panic in a call of the source leaves the source as it was.
        let lent = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let source = lent
            .as_deref()
            .ok_or_else(|| io::Error::other("the source was given back"))?;
        call(source)
    }
}

impl ReadOnlyFileSystem for LentSource {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.with(|source| source.entry(path))
    }

    fn list_folder(&self, path: &Path) -> io::Result<Vec<FolderEntry>> {
        self.with(|source| source.list_folder(path))
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        self.with(|source| source.file_size(path))
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        self.with(|source| source.open_read(path))
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        self.with(|source| source.modification_time(path))
    }
}

/// The file system of the destinations, read as a source on the source thread.
struct SharedSource(Arc<dyn FileSystem + Send + Sync>);

impl ReadOnlyFileSystem for SharedSource {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.0.entry(path)
    }

    fn list_folder(&self, path: &Path) -> io::Result<Vec<FolderEntry>> {
        self.0.list_folder(path)
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        self.0.file_size(path)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        self.0.open_read(path)
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        self.0.modification_time(path)
    }
}
