// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A [`Trash`] in a [`MemoryFileSystem`], for tests.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::testing::MemoryFileSystem;
use crate::{EntryKind, FileSystem, ReadOnlyFileSystem, Trash, not_a_file};

/// A [`Trash`] that moves files to a folder of a [`MemoryFileSystem`], for tests
/// (ADR 0043).
///
/// A file of the same name as one already in the trash gets a number before its
/// extension, `DSCF0001.2.RAF`. [`MemoryTrash::refuse`] makes the trash refuse a
/// file, as a real trash refuses one it may not move. Clones share one trash.
///
/// ```
/// use std::path::Path;
///
/// use domain::Timestamp;
/// use files::Trash;
/// use files::testing::{MemoryFileSystem, MemoryTrash};
///
/// let file_system = MemoryFileSystem::new();
/// file_system.add_file(Path::new("/photos/A.RAF"), b"raw", Timestamp::from_unix_seconds(0));
/// let trash = MemoryTrash::new(file_system.clone(), "/trash");
///
/// let in_trash = trash.move_to_trash(Path::new("/photos/A.RAF"))?;
/// assert_eq!(file_system.content(&in_trash), Some(b"raw".to_vec()));
/// assert_eq!(trash.moved(), vec![Path::new("/photos/A.RAF").to_path_buf()]);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct MemoryTrash {
    file_system: MemoryFileSystem,
    folder: PathBuf,
    state: Arc<Mutex<TrashState>>,
}

#[derive(Debug, Default)]
struct TrashState {
    refused: BTreeSet<PathBuf>,
    moved: Vec<PathBuf>,
}

impl MemoryTrash {
    /// Creates a trash that moves files of `file_system` to `folder`.
    #[must_use]
    pub fn new(file_system: MemoryFileSystem, folder: impl Into<PathBuf>) -> Self {
        MemoryTrash {
            file_system,
            folder: folder.into(),
            state: Arc::default(),
        }
    }

    /// Makes the trash refuse the file `path` with
    /// [`io::ErrorKind::PermissionDenied`]; the file stays.
    pub fn refuse(&self, path: &Path) {
        self.lock().refused.insert(path.to_path_buf());
    }

    /// The paths of the files moved to the trash, in the order they were moved.
    #[must_use]
    pub fn moved(&self) -> Vec<PathBuf> {
        self.lock().moved.clone()
    }

    fn lock(&self) -> MutexGuard<'_, TrashState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn free_place(&self, path: &Path) -> io::Result<PathBuf> {
        let stem = path.file_stem().unwrap_or_default();
        for number in 1_u32.. {
            let mut name = OsString::from(stem);
            if number > 1 {
                name.push(format!(".{number}"));
            }
            if let Some(extension) = path.extension() {
                name.push(".");
                name.push(extension);
            }
            let candidate = self.folder.join(name);
            if self.file_system.entry(&candidate)?.is_none() {
                return Ok(candidate);
            }
        }
        Err(io::Error::other("the trash has no free name"))
    }
}

impl Trash for MemoryTrash {
    fn move_to_trash(&self, path: &Path) -> io::Result<PathBuf> {
        match self.file_system.entry(path)? {
            None => return Err(io::Error::from(io::ErrorKind::NotFound)),
            Some(EntryKind::File) => {}
            Some(_) => return Err(not_a_file(path)),
        }
        if self.lock().refused.contains(path) {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied));
        }
        self.file_system.add_folder(&self.folder);
        let in_trash = self.free_place(path)?;
        self.file_system.rename_no_replace(path, &in_trash)?;
        self.lock().moved.push(path.to_path_buf());
        Ok(in_trash)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use domain::Timestamp;

    use super::*;
    use crate::testing::{TrashUnderTest, check_trash};

    #[test]
    fn conforms_to_the_port() {
        let file_system = MemoryFileSystem::new();
        file_system.add_folder(Path::new("/root"));
        let trash = MemoryTrash::new(file_system.clone(), "/trash");
        let write_file = |path: &Path, content: &[u8]| {
            file_system.add_file(path, content, Timestamp::from_unix_seconds(0));
            Ok(())
        };
        let read_file = |path: &Path| {
            let mut content = Vec::new();
            file_system.open_read(path)?.read_to_end(&mut content)?;
            Ok(content)
        };
        let remove_file = |path: &Path| file_system.remove_file(path);

        let failures = check_trash(&TrashUnderTest {
            trash: &trash,
            root: Path::new("/root"),
            write_file: &write_file,
            read_file: &read_file,
            remove_file: &remove_file,
        });

        assert_eq!(failures, Vec::<String>::new());
    }

    #[test]
    fn a_refused_file_stays_and_is_not_recorded() {
        let file_system = MemoryFileSystem::new();
        let raw = Path::new("/photos/A.RAF");
        file_system.add_file(raw, b"raw", Timestamp::from_unix_seconds(0));
        let trash = MemoryTrash::new(file_system.clone(), "/trash");
        trash.refuse(raw);

        let error = trash.move_to_trash(raw).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(file_system.content(raw), Some(b"raw".to_vec()));
        assert_eq!(trash.moved(), Vec::<PathBuf>::new());
    }

    #[test]
    fn a_second_file_of_the_same_name_gets_a_number_before_its_extension() {
        let file_system = MemoryFileSystem::new();
        file_system.add_file(Path::new("/a/A.RAF"), b"a", Timestamp::from_unix_seconds(0));
        file_system.add_file(Path::new("/b/A.RAF"), b"b", Timestamp::from_unix_seconds(0));
        let trash = MemoryTrash::new(file_system, "/trash");

        trash.move_to_trash(Path::new("/a/A.RAF")).unwrap();
        let second = trash.move_to_trash(Path::new("/b/A.RAF")).unwrap();

        assert_eq!(second, Path::new("/trash/A.2.RAF"));
    }
}
