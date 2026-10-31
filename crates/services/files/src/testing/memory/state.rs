// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The state of a [`MemoryFileSystem`](super::MemoryFileSystem): its current and
//! durable entries and files.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use domain::Timestamp;

use super::Fault;
use crate::{EntryKind, FolderEntry};

#[derive(Debug, Default)]
pub(super) struct State {
    pub(super) entries: BTreeMap<PathBuf, Entry>,
    pub(super) durable_entries: BTreeMap<PathBuf, Entry>,
    pub(super) files: BTreeMap<FileId, FileData>,
    pub(super) next_file_id: FileId,
    pub(super) faults: BTreeMap<PathBuf, Fault>,
    pub(super) operations: usize,
    pub(super) stop_after: Option<usize>,
}

pub(super) type FileId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Entry {
    Folder,
    File(FileId),
    SymbolicLink,
}

#[derive(Debug, Clone)]
pub(super) struct FileData {
    pub(super) content: Vec<u8>,
    pub(super) modified: Timestamp,
    pub(super) durable_content: Vec<u8>,
    pub(super) durable_modified: Timestamp,
}

impl FileData {
    pub(super) fn empty() -> Self {
        FileData {
            content: Vec::new(),
            modified: Timestamp::UNIX_EPOCH,
            durable_content: Vec::new(),
            durable_modified: Timestamp::UNIX_EPOCH,
        }
    }
}

/// A poisoned lock only means another test thread panicked; the state is plain data.
pub(super) fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl State {
    pub(super) fn count_operation(&mut self) -> io::Result<()> {
        if self
            .stop_after
            .is_some_and(|limit| self.operations >= limit)
        {
            return Err(io::Error::other("the program stopped"));
        }
        self.operations += 1;
        Ok(())
    }

    pub(super) fn add_folders(&mut self, path: &Path) {
        for folder in path.ancestors().filter(|folder| !is_root(folder)) {
            self.entries.insert(folder.to_path_buf(), Entry::Folder);
            self.durable_entries
                .insert(folder.to_path_buf(), Entry::Folder);
        }
    }

    pub(super) fn new_file(&mut self, content: Vec<u8>, modified: Timestamp) -> FileId {
        let id = self.next_file_id;
        self.next_file_id += 1;
        let data = FileData {
            content,
            modified,
            durable_content: Vec::new(),
            durable_modified: modified,
        };
        self.files.insert(id, data);
        id
    }

    pub(super) fn make_file_durable(&mut self, id: FileId) {
        let file = self.file_mut(id);
        file.durable_content.clone_from(&file.content);
        file.durable_modified = file.modified;
    }

    pub(super) fn file(&self, id: FileId) -> &FileData {
        &self.files[&id]
    }

    pub(super) fn file_mut(&mut self, id: FileId) -> &mut FileData {
        // Entries only ever hold ids of files in `files`; the default is never used.
        self.files.entry(id).or_insert_with(FileData::empty)
    }

    pub(super) fn entry_kind(&self, path: &Path) -> Option<EntryKind> {
        if is_root(path) {
            return Some(EntryKind::Folder);
        }
        match self.entries.get(path)? {
            Entry::Folder => Some(EntryKind::Folder),
            Entry::File(_) => Some(EntryKind::File),
            Entry::SymbolicLink => Some(EntryKind::SymbolicLink),
        }
    }

    /// The entries directly in `folder`, by name.
    pub(super) fn children(&self, folder: &Path) -> io::Result<Vec<FolderEntry>> {
        match self.entry_kind(folder) {
            Some(EntryKind::Folder) => {}
            Some(_) => return Err(not_a_folder(folder)),
            None => return Err(not_found(folder)),
        }
        if let Some(Fault::ListFails) = self.faults.get(folder) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("{} cannot be read", folder.display()),
            ));
        }
        let children = self
            .entries
            .keys()
            .filter(|path| is_child_of(path, folder))
            .filter_map(|path| {
                Some(FolderEntry {
                    name: path.file_name()?.to_os_string(),
                    kind: self.entry_kind(path)?,
                })
            });
        Ok(children.collect())
    }

    pub(super) fn file_id(&self, path: &Path) -> io::Result<FileId> {
        match self.entries.get(path) {
            Some(Entry::File(id)) => Ok(*id),
            Some(Entry::Folder) => Err(io::Error::other(format!("{} is a folder", path.display()))),
            Some(Entry::SymbolicLink) => Err(symbolic_link(path)),
            None => Err(not_found(path)),
        }
    }

    pub(super) fn require_parent_folder(&self, path: &Path) -> io::Result<()> {
        let parent = path.parent().unwrap_or(Path::new(""));
        match self.entry_kind(parent) {
            Some(EntryKind::Folder) => Ok(()),
            _ => Err(not_found(parent)),
        }
    }

    pub(super) fn require_absent(&self, path: &Path) -> io::Result<()> {
        if self.entry_kind(path).is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} already exists", path.display()),
            ));
        }
        Ok(())
    }

    pub(super) fn sync_folder(&mut self, folder: &Path) -> io::Result<()> {
        if self.entry_kind(folder) != Some(EntryKind::Folder) {
            return Err(not_found(folder));
        }
        self.durable_entries
            .retain(|path, _| !is_child_of(path, folder));
        let children = self
            .entries
            .iter()
            .filter(|(path, _)| is_child_of(path, folder))
            .map(|(path, entry)| (path.clone(), *entry));
        self.durable_entries.extend(children.collect::<Vec<_>>());
        Ok(())
    }

    pub(super) fn fault_for(&mut self, path: &Path) -> Option<Fault> {
        self.faults.get(path).cloned()
    }
}

/// The durable entries whose folders all exist, as they are found after a crash.
pub(super) fn reachable_entries(entries: &BTreeMap<PathBuf, Entry>) -> BTreeMap<PathBuf, Entry> {
    let mut by_depth: Vec<(&PathBuf, &Entry)> = entries.iter().collect();
    by_depth.sort_by_key(|(path, _)| path.components().count());
    let mut reachable = BTreeMap::new();
    for (path, entry) in by_depth {
        let parent = path.parent().unwrap_or(Path::new(""));
        if is_root(parent) || reachable.get(parent) == Some(&Entry::Folder) {
            reachable.insert(path.clone(), *entry);
        }
    }
    reachable
}

pub(super) fn is_root(path: &Path) -> bool {
    path.parent().is_none() || path.as_os_str().is_empty()
}

fn is_child_of(path: &Path, folder: &Path) -> bool {
    let parent = path.parent().unwrap_or(Path::new(""));
    parent == folder || (is_root(parent) && is_root(folder))
}

/// The memory file system never follows symbolic links; they exist to be listed.
pub(super) fn symbolic_link(path: &Path) -> io::Error {
    io::Error::other(format!(
        "{} is a symbolic link, which this file system does not follow",
        path.display()
    ))
}

fn not_a_folder(path: &Path) -> io::Error {
    io::Error::other(format!("{} is not a folder", path.display()))
}

fn not_found(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("{} does not exist", path.display()),
    )
}
