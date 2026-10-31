// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A [`FileSystem`] in memory that can lose unsynced writes and inject faults.

mod handles;
mod state;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use domain::Timestamp;

use crate::{EntryKind, FileSystem, FolderEntry, ReadSeek, WriteFile};

use handles::{MemoryReader, MemoryWriter};
use state::{Entry, State, lock, reachable_entries, symbolic_link};

/// A fault that [`MemoryFileSystem`] injects for one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// Reading the file fails once `after_bytes` bytes were read.
    ReadFails {
        /// How many bytes can be read before the error.
        after_bytes: usize,
    },
    /// Writing the file fails once it holds `after_bytes` bytes, as on a full device.
    WriteFails {
        /// How many bytes the file can hold.
        after_bytes: usize,
    },
    /// Reading the file returns different bytes than it holds: the last byte is
    /// inverted.
    ReadReturnsDifferentBytes,
    /// Listing the folder fails with [`io::ErrorKind::PermissionDenied`], as for a
    /// folder the user may not read.
    ListFails,
    /// The first write to the file makes another program create the file `path`
    /// with `content`.
    OtherProgramCreates {
        /// The file the other program creates.
        path: PathBuf,
        /// Its content.
        content: Vec<u8>,
    },
}

/// A [`FileSystem`] in memory, for tests.
///
/// It models a storage device with a cache: content, modification times and
/// folder entries change at once, but are durable only once synced. The content and
/// modification time of a file are durable after [`WriteFile::sync`], and the
/// entries of a folder (created, renamed and removed names) after
/// [`FileSystem::sync_folder`]. [`MemoryFileSystem::crash`] drops everything that
/// was not durable.
///
/// Files, folders and symbolic links added with [`MemoryFileSystem::add_file`],
/// [`MemoryFileSystem::add_folder`] and [`MemoryFileSystem::add_symbolic_link`] are
/// durable at once. Symbolic links are only listed: opening one, or asking for its
/// size or modification time, fails. Clones share one file
/// system.
///
/// ```
/// use std::io::Write;
/// use std::path::Path;
///
/// use domain::Timestamp;
/// use files::FileSystem;
/// use files::testing::MemoryFileSystem;
///
/// let file_system = MemoryFileSystem::new();
/// file_system.add_folder(Path::new("/destination"));
/// let mut file = file_system.create_file(Path::new("/destination/photo.jpg"))?;
/// file.write_all(b"jpeg")?;
///
/// file_system.crash();
/// assert_eq!(file_system.content(Path::new("/destination/photo.jpg")), None);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct MemoryFileSystem {
    state: Arc<Mutex<State>>,
}

impl MemoryFileSystem {
    /// Creates an empty file system that holds only its root folder.
    #[must_use]
    pub fn new() -> Self {
        MemoryFileSystem::default()
    }

    /// Adds the folder `path` and its missing parents, all durable.
    pub fn add_folder(&self, path: &Path) {
        let mut state = self.lock();
        state.add_folders(path);
    }

    /// Adds the file `path` with `content` and modification time `modified`, and its
    /// missing parent folders, all durable. An existing file is replaced.
    pub fn add_file(&self, path: &Path, content: &[u8], modified: Timestamp) {
        let mut state = self.lock();
        if let Some(parent) = path.parent() {
            state.add_folders(parent);
        }
        let id = state.new_file(content.to_vec(), modified);
        state.make_file_durable(id);
        state.entries.insert(path.to_path_buf(), Entry::File(id));
        state
            .durable_entries
            .insert(path.to_path_buf(), Entry::File(id));
    }

    /// Adds a symbolic link at `path` and its missing parent folders, all durable.
    /// What it points to does not matter, because this file system never follows it.
    pub fn add_symbolic_link(&self, path: &Path) {
        let mut state = self.lock();
        if let Some(parent) = path.parent() {
            state.add_folders(parent);
        }
        state
            .entries
            .insert(path.to_path_buf(), Entry::SymbolicLink);
        state
            .durable_entries
            .insert(path.to_path_buf(), Entry::SymbolicLink);
    }

    /// Returns the current content of the file `path`, or `None` if there is none.
    #[must_use]
    pub fn content(&self, path: &Path) -> Option<Vec<u8>> {
        let state = self.lock();
        let id = state.file_id(path).ok()?;
        Some(state.file(id).content.clone())
    }

    /// Returns the current modification time of the file `path`, or `None` if
    /// there is none.
    #[must_use]
    pub fn modified(&self, path: &Path) -> Option<Timestamp> {
        let state = self.lock();
        let id = state.file_id(path).ok()?;
        Some(state.file(id).modified)
    }

    /// Returns the paths of all files and folders, in order.
    #[must_use]
    pub fn paths(&self) -> Vec<PathBuf> {
        self.lock().entries.keys().cloned().collect()
    }

    /// Simulates a crash of the storage device: every change that was not synced is
    /// lost, and only durable files and folders remain.
    pub fn crash(&self) {
        let mut state = self.lock();
        let durable = reachable_entries(&state.durable_entries);
        state.durable_entries.clone_from(&durable);
        state.entries = durable;
        for file in state.files.values_mut() {
            file.content.clone_from(&file.durable_content);
            file.modified = file.durable_modified;
        }
    }

    /// Injects `fault` for the file `path`, replacing an earlier fault for it.
    pub fn inject_fault(&self, path: &Path, fault: Fault) {
        self.lock().faults.insert(path.to_path_buf(), fault);
    }

    /// Lets the next `operations` operations succeed and fails every later one, as if
    /// the program had stopped there.
    ///
    /// Each call of a [`FileSystem`] method and each read, write and sync of an open
    /// file is one operation.
    pub fn stop_after_operations(&self, operations: usize) {
        let mut state = self.lock();
        state.stop_after = Some(state.operations + operations);
    }

    /// Returns how many operations were made so far.
    #[must_use]
    pub fn operations(&self) -> usize {
        self.lock().operations
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        lock(&self.state)
    }

    fn operate<T>(&self, operation: impl FnOnce(&mut State) -> io::Result<T>) -> io::Result<T> {
        let mut state = self.lock();
        state.count_operation()?;
        operation(&mut state)
    }
}

impl FileSystem for MemoryFileSystem {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.operate(|state| Ok(state.entry_kind(path)))
    }

    fn list_folder(&self, path: &Path) -> io::Result<Vec<FolderEntry>> {
        self.operate(|state| state.children(path))
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        self.operate(|state| {
            let id = state.file_id(path)?;
            u64::try_from(state.file(id).content.len()).map_err(io::Error::other)
        })
    }

    fn create_folder(&self, path: &Path) -> io::Result<()> {
        self.operate(|state| {
            state.require_absent(path)?;
            state.require_parent_folder(path)?;
            state.entries.insert(path.to_path_buf(), Entry::Folder);
            Ok(())
        })
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        self.operate(|state| {
            let id = state.file_id(path)?;
            let mut content = state.file(id).content.clone();
            let mut readable = content.len();
            match state.fault_for(path) {
                Some(Fault::ReadReturnsDifferentBytes) => {
                    if let Some(last) = content.last_mut() {
                        *last = !*last;
                    }
                }
                Some(Fault::ReadFails { after_bytes }) => readable = after_bytes,
                _ => {}
            }
            let reader = MemoryReader {
                state: Arc::clone(&self.state),
                content,
                position: 0,
                readable,
            };
            Ok(Box::new(reader) as Box<dyn ReadSeek>)
        })
    }

    fn create_file(&self, path: &Path) -> io::Result<Box<dyn WriteFile>> {
        self.operate(|state| {
            state.require_parent_folder(path)?;
            let id = match state.entries.get(path) {
                Some(Entry::File(id)) => *id,
                Some(Entry::Folder) => {
                    return Err(io::Error::other(format!("{} is a folder", path.display())));
                }
                Some(Entry::SymbolicLink) => return Err(symbolic_link(path)),
                None => {
                    let id = state.new_file(Vec::new(), Timestamp::UNIX_EPOCH);
                    state.entries.insert(path.to_path_buf(), Entry::File(id));
                    id
                }
            };
            state.file_mut(id).content.clear();
            let writer = MemoryWriter {
                state: Arc::clone(&self.state),
                id,
                fault: state.fault_for(path),
            };
            Ok(Box::new(writer) as Box<dyn WriteFile>)
        })
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        self.operate(|state| {
            let id = state.file_id(path)?;
            Ok(state.file(id).modified)
        })
    }

    fn rename_no_replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.operate(|state| {
            let id = state.file_id(from)?;
            state.require_absent(to)?;
            state.require_parent_folder(to)?;
            state.entries.remove(from);
            state.entries.insert(to.to_path_buf(), Entry::File(id));
            Ok(())
        })
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.operate(|state| {
            state.file_id(path)?;
            state.entries.remove(path);
            Ok(())
        })
    }

    fn sync_folder(&self, path: &Path) -> io::Result<()> {
        self.operate(|state| state.sync_folder(path))
    }
}
