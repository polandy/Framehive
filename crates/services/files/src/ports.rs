// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The ports of the files service (ADR 0020 §4).

use std::ffi::OsString;
use std::io::{self, Read, Seek, Write};
use std::path::Path;

use domain::{Checksum, Timestamp};

/// What a path names in a [`FileSystem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// A regular file.
    File,
    /// A folder.
    Folder,
    /// A symbolic link. The port never follows it when it inspects or lists a path,
    /// so a scan cannot loop or leave its source (IMPORT-005).
    SymbolicLink,
    /// Anything else, such as a device or a socket.
    Other,
}

/// One entry of a folder, as [`ReadOnlyFileSystem::list_folder`] returns it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FolderEntry {
    /// The entry's name within its folder, without the folder's path.
    pub name: OsString,
    /// What the entry is; a symbolic link is [`EntryKind::SymbolicLink`], not what it
    /// points to.
    pub kind: EntryKind,
}

/// Read-only port for the files and folders of a storage device (ADR 0020 §4,
/// ADR 0034).
///
/// The import reads its source through it, so a source that cannot be written,
/// such as a camera, can implement it alone. Every [`FileSystem`] is one. Every
/// implementation must pass
/// [`testing::check_read_only_file_system`](crate::testing::check_read_only_file_system).
///
/// Errors are [`io::Error`]s. Where the contract names an [`io::ErrorKind`], callers
/// rely on it.
pub trait ReadOnlyFileSystem {
    /// Returns what `path` names, or `None` if nothing exists there. A symbolic link
    /// is [`EntryKind::SymbolicLink`]; it is not followed.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be inspected.
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>>;

    /// Returns the entries of the folder `path`, in no particular order, without `.`
    /// and `..`. Symbolic links are listed as [`EntryKind::SymbolicLink`] and not
    /// followed.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if nothing exists at `path`, and an error
    /// if `path` is not a folder or cannot be read.
    fn list_folder(&self, path: &Path) -> io::Result<Vec<FolderEntry>>;

    /// Returns the size in bytes of the file `path`.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if no file exists at `path`, and an error
    /// if `path` is a folder.
    fn file_size(&self, path: &Path) -> io::Result<u64>;

    /// Opens the file `path` for reading, from the start or from any offset.
    ///
    /// Seeking follows [`Seek`]: a position past the end reads nothing, and a position
    /// before the start is an error. Metadata readers seek to the byte ranges they
    /// need instead of reading a whole RAW file (ADR 0021).
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if no file exists at `path`.
    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>>;

    /// Returns the modification time of the file `path`.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if no file exists at `path`.
    fn modification_time(&self, path: &Path) -> io::Result<Timestamp>;
}

/// Port for the files and folders of a storage device (ADR 0020 §4).
///
/// `adapter-fs` implements it over `std::fs`; tests use
/// [`testing::MemoryFileSystem`](crate::testing::MemoryFileSystem). Every
/// implementation must pass
/// [`testing::check_file_system`](crate::testing::check_file_system).
///
/// Errors are [`io::Error`]s. Where the contract names an [`io::ErrorKind`], callers
/// rely on it.
pub trait FileSystem: ReadOnlyFileSystem {
    /// Creates the folder `path`. Its parent folder must exist.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::AlreadyExists`] if something exists at `path`, and
    /// [`io::ErrorKind::NotFound`] if the parent folder is missing.
    fn create_folder(&self, path: &Path) -> io::Result<()>;

    /// Creates the file `path` for writing, or empties it if it exists. Its parent
    /// folder must exist.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if the parent folder is missing, and an
    /// error if `path` names something that is not a file.
    fn create_file(&self, path: &Path) -> io::Result<Box<dyn WriteFile>>;

    /// Renames the file `from` to `to`, which must not exist. It never replaces a
    /// file: if something exists at `to`, or appears there during the call, both
    /// files stay unchanged.
    ///
    /// The new name is durable only after [`FileSystem::sync_folder`] on the folder
    /// of `to`.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::AlreadyExists`] if something exists at `to`, and
    /// [`io::ErrorKind::NotFound`] if no file exists at `from`.
    fn rename_no_replace(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Renames the file `from` over the existing file `to` in one step: `to` always
    /// names either the old or the new file. The new file takes over the permissions
    /// of the old one, so a replace never changes them (CORE-009).
    ///
    /// The new name is durable only after [`FileSystem::sync_folder`] on the folder
    /// of `to`.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if no file exists at `from` or at `to`, and
    /// [`io::ErrorKind::PermissionDenied`] if the file at `to` is read-only or the
    /// user may not write it; a read-only file is refused for every user alike. Both
    /// files then stay unchanged.
    fn replace_file(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Removes the file `path`.
    ///
    /// # Errors
    ///
    /// Returns [`io::ErrorKind::NotFound`] if no file exists at `path`.
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Writes the entries of the folder `path` to the storage device, so that files
    /// created, renamed or removed in it survive a crash.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder does not exist or cannot be synced.
    fn sync_folder(&self, path: &Path) -> io::Result<()>;
}

/// A file opened for reading by [`ReadOnlyFileSystem::open_read`]: it reads and seeks.
///
/// Every type that is [`Read`] and [`Seek`] is a `ReadSeek`, such as
/// [`std::fs::File`] and [`std::io::Cursor`].
pub trait ReadSeek: Read + Seek {}

impl<T: Read + Seek + ?Sized> ReadSeek for T {}

/// A file opened for writing by [`FileSystem::create_file`].
pub trait WriteFile: Write {
    /// Sets the file's modification time. Later writes may change it again.
    ///
    /// # Errors
    ///
    /// Returns an error if the time cannot be set.
    fn set_modification_time(&mut self, time: Timestamp) -> io::Result<()>;

    /// Writes the file's content and modification time to the storage device, so
    /// that they survive a crash. Its name is durable only after
    /// [`FileSystem::sync_folder`].
    ///
    /// # Errors
    ///
    /// Returns an error if the data cannot be written to the device.
    fn sync(&mut self) -> io::Result<()>;
}

/// Port for a hash function that computes a [`Checksum`] (ADR 0020 §4).
///
/// `adapter-blake3` implements it with BLAKE3; tests use
/// [`testing::FnvHasher`](crate::testing::FnvHasher). Every implementation must pass
/// [`testing::check_hasher`](crate::testing::check_hasher).
pub trait Hasher {
    /// Starts a new hash over no bytes.
    fn start(&self) -> Box<dyn HashState>;
}

/// A hash in progress, started by [`Hasher::start`].
pub trait HashState {
    /// Adds `bytes` to the hashed content. Adding content in pieces gives the same
    /// checksum as adding it at once.
    fn update(&mut self, bytes: &[u8]);

    /// Returns the checksum of all bytes added, with the hasher's algorithm id.
    fn finish(self: Box<Self>) -> Checksum;
}

/// The identity of a file system: two folders with the same identity share one free
/// space (IMPORT-012, ADR 0033). On Unix it is the device number `st_dev`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileSystemId(u64);

impl FileSystemId {
    /// The file system with the identity `id`.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        FileSystemId(id)
    }

    /// The identity as a number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// What [`FreeSpace::free_space`] answers for a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpaceOnFileSystem {
    /// The file system the folder lies on.
    pub file_system: FileSystemId,
    /// The bytes an unprivileged process may still write there.
    pub available_bytes: u64,
}

/// Port for the free space of the file system a folder lies on (ADR 0033).
///
/// `adapter-free-space` implements it with `statvfs`; tests use
/// [`testing::MemoryFreeSpace`](crate::testing::MemoryFreeSpace). Every
/// implementation must pass
/// [`testing::check_free_space`](crate::testing::check_free_space).
pub trait FreeSpace {
    /// Returns the free space and the identity of the file system `folder` lies on.
    /// A folder that does not exist yet is answered for its nearest existing parent
    /// folder, because the import creates missing destinations itself.
    ///
    /// # Errors
    ///
    /// Returns an error if the free space cannot be read, for example on a file
    /// system that does not report it.
    fn free_space(&self, folder: &Path) -> io::Result<SpaceOnFileSystem>;
}
