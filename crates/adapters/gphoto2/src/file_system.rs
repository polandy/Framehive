// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! [`CameraFileSystem`]: an opened camera as a [`ReadOnlyFileSystem`] (ADR 0034 §2).

use std::ffi::{CString, OsStr};
use std::io::{self, ErrorKind};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use domain::Timestamp;
use files::{EntryKind, FolderEntry, ReadOnlyFileSystem, ReadSeek};

use crate::library::{GpError, c_string};
use crate::reader::CameraReader;
use crate::session::{FileInfo, Session};

/// The file system of an opened camera: the root `/` holds one folder per storage.
/// Dropping it, and every reader opened from it, releases the camera.
pub(crate) struct CameraFileSystem {
    session: Arc<Mutex<Session>>,
}

impl CameraFileSystem {
    pub(crate) fn new(session: Session) -> Self {
        Self {
            session: Arc::new(Mutex::new(session)),
        }
    }

    fn session(&self) -> io::Result<MutexGuard<'_, Session>> {
        lock(&self.session)
    }

    /// What libgphoto2 reports about the file `path`, which must be a file.
    fn file_info(&self, path: &Path) -> io::Result<(CString, CString, FileInfo)> {
        let Some((folder, name)) = split(path)? else {
            return Err(is_a_folder(path));
        };
        let mut session = self.session()?;
        match entry_in(&mut session, &folder, &name) {
            Ok(Some(EntryKind::File)) => {}
            Ok(Some(_)) => return Err(is_a_folder(path)),
            Ok(None) => return Err(not_found(path)),
            Err(error) => return Err(io_error("inspect", path, error)),
        }
        let info = session
            .with_fresh_listings(|session| session.file_info(&folder, &name))
            .map_err(|error| io_error("read the information of", path, error))?;
        Ok((folder, name, info))
    }
}

impl ReadOnlyFileSystem for CameraFileSystem {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        let Some((folder, name)) = split(path)? else {
            return Ok(Some(EntryKind::Folder));
        };
        entry_in(&mut *self.session()?, &folder, &name)
            .map_err(|error| io_error("inspect", path, error))
    }

    fn list_folder(&self, path: &Path) -> io::Result<Vec<FolderEntry>> {
        let folder = c_path(path)?;
        let mut session = self.session()?;
        let listing = session.with_fresh_listings(|session| {
            Ok((session.list_folders(&folder)?, session.list_files(&folder)?))
        });
        match listing {
            Ok((folders, files)) => Ok(folder_entries(folders, files)),
            Err(error) if error.is_not_found() => Err(missing_folder(&mut session, path)),
            Err(error) => Err(io_error("list", path, error)),
        }
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        let (_, _, info) = self.file_info(path)?;
        info.size.ok_or_else(|| not_reported("size", path))
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        let (folder, name, info) = self.file_info(path)?;
        let size = info.size.ok_or_else(|| not_reported("size", path))?;
        Ok(Box::new(CameraReader::new(
            Arc::clone(&self.session),
            folder,
            name,
            size,
            path,
        )))
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        let (_, _, info) = self.file_info(path)?;
        info.modified
            .map(Timestamp::from_unix_seconds)
            .ok_or_else(|| not_reported("modification time", path))
    }
}

/// Locks the session of a camera.
pub(crate) fn lock(session: &Mutex<Session>) -> io::Result<MutexGuard<'_, Session>> {
    session.lock().map_err(|_| {
        io::Error::other("the camera is in an unknown state after a failure in another thread")
    })
}

/// What `name` in `folder` is, or `None` if `folder` holds nothing of that name or does
/// not exist.
fn entry_in(
    session: &mut Session,
    folder: &CString,
    name: &CString,
) -> Result<Option<EntryKind>, GpError> {
    let name = OsStr::from_bytes(name.as_bytes());
    let folders = match session.with_fresh_listings(|session| session.list_folders(folder)) {
        Ok(folders) => folders,
        Err(error) if error.is_not_found() => return Ok(None),
        Err(error) => return Err(error),
    };
    if folders.iter().any(|folder| folder == name) {
        return Ok(Some(EntryKind::Folder));
    }
    let files = session.list_files(folder)?;
    Ok(files
        .iter()
        .any(|file| file == name)
        .then_some(EntryKind::File))
}

/// The error for listing `path`, which is not a folder.
fn missing_folder(session: &mut Session, path: &Path) -> io::Error {
    let is_file = match split(path) {
        Ok(Some((folder, name))) => entry_in(session, &folder, &name) == Ok(Some(EntryKind::File)),
        _ => false,
    };
    if is_file {
        io::Error::new(
            ErrorKind::NotADirectory,
            format!("{} is a file, not a folder", path.display()),
        )
    } else {
        not_found(path)
    }
}

fn folder_entries(
    folders: Vec<std::ffi::OsString>,
    files: Vec<std::ffi::OsString>,
) -> Vec<FolderEntry> {
    let folders = folders.into_iter().map(|name| FolderEntry {
        name,
        kind: EntryKind::Folder,
    });
    let files = files.into_iter().map(|name| FolderEntry {
        name,
        kind: EntryKind::File,
    });
    folders.chain(files).collect()
}

/// Splits an absolute path into its folder and its name, or returns `None` for the
/// root `/`.
fn split(path: &Path) -> io::Result<Option<(CString, CString)>> {
    let folder_path = c_path(path)?;
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return if folder_path.as_bytes() == b"/" {
            Ok(None)
        } else {
            Err(invalid_path(path))
        };
    };
    let folder = c_path(parent)?;
    let name = c_string(name.as_bytes()).map_err(|_| invalid_path(path))?;
    Ok(Some((folder, name)))
}

/// `path` as a C string; a camera path is absolute and has no NUL byte.
fn c_path(path: &Path) -> io::Result<CString> {
    if !path.is_absolute() {
        return Err(invalid_path(path));
    }
    c_string(path.as_os_str().as_bytes()).map_err(|_| invalid_path(path))
}

fn invalid_path(path: &Path) -> io::Error {
    io::Error::new(
        ErrorKind::InvalidInput,
        format!("{} is not an absolute path on the camera", path.display()),
    )
}

fn not_found(path: &Path) -> io::Error {
    io::Error::new(
        ErrorKind::NotFound,
        format!("{} does not exist on the camera", path.display()),
    )
}

fn is_a_folder(path: &Path) -> io::Error {
    io::Error::new(
        ErrorKind::IsADirectory,
        format!("{} is a folder, not a file", path.display()),
    )
}

fn not_reported(what: &str, path: &Path) -> io::Error {
    io::Error::new(
        ErrorKind::Unsupported,
        format!("the camera reports no {what} for {}", path.display()),
    )
}

/// The error of a libgphoto2 call that failed to `operation` `path`.
pub(crate) fn io_error(operation: &str, path: &Path, error: GpError) -> io::Error {
    let kind = if error.is_not_found() {
        ErrorKind::NotFound
    } else if error.is_busy() {
        ErrorKind::ResourceBusy
    } else if error.is_not_supported() {
        ErrorKind::Unsupported
    } else {
        ErrorKind::Other
    };
    io::Error::new(
        kind,
        format!(
            "cannot {operation} {} on the camera: {}",
            path.display(),
            error.message()
        ),
    )
}
