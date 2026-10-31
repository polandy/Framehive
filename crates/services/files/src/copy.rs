// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The verified copy of one file (IMPORT-004, ADR 0020 §3).

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use domain::Checksum;

use crate::checksum::{BUFFER_SIZE, checksum_file, read_some};
use crate::{CleanupFailure, CopyError, CopyErrorKind, CopyStep, EntryKind, FileSystem, Hasher};

/// Copies `source` to `target` and verifies the copy (glossary: Verified copy).
///
/// The steps follow IMPORT-004:
///
/// 1. Missing folders of `target` are created, and each is synced into its parent.
/// 2. `source` is read once; its bytes are hashed and written to `<target>.part`,
///    which replaces a `.part` file left by an interrupted import.
/// 3. The `.part` file gets the source's modification time and is synced, then
///    renamed to `target` without replacing a file, and the folder is synced.
/// 4. `target` is read back, and its checksum is compared with the source's.
///
/// Returns the source's checksum once all steps succeeded. Nothing in the source is
/// written.
///
/// On a failure, the `.part` file is removed, and so is a target this call renamed
/// into place, because it is not verified. A file that another program created at
/// `target` is never touched.
///
/// ```
/// use std::path::Path;
///
/// use domain::Timestamp;
/// use files::copy_verified;
/// use files::testing::{FnvHasher, MemoryFileSystem};
///
/// let file_system = MemoryFileSystem::new();
/// file_system.add_file(Path::new("/card/DSCF0001.RAF"), b"raw", Timestamp::UNIX_EPOCH);
/// let target = Path::new("/photos/2026/DSCF0001.RAF");
///
/// let checksum = copy_verified(&file_system, &FnvHasher, Path::new("/card/DSCF0001.RAF"), target)?;
///
/// assert_eq!(file_system.content(target).as_deref(), Some(&b"raw"[..]));
/// assert_eq!(checksum.algorithm(), "fnv");
/// # Ok::<(), files::CopyError>(())
/// ```
///
/// # Errors
///
/// Returns a [`CopyError`] with the step that failed and why:
/// [`CopyErrorKind::TargetExists`] if a file appeared at `target` during the copy,
/// [`CopyErrorKind::ChecksumMismatch`] if the target read back differs from the
/// source, and [`CopyErrorKind::Io`] for any error of the file system.
pub fn copy_verified(
    file_system: &dyn FileSystem,
    hasher: &dyn Hasher,
    source: &Path,
    target: &Path,
) -> Result<Checksum, CopyError> {
    let part = part_path(target);
    let copy = Copy {
        file_system,
        hasher,
        source,
        target,
        part: &part,
    };
    copy.create_missing_folders()
        .map_err(|failure| failure.into_error(None))?;
    let source_checksum = copy
        .write_part()
        .and_then(|checksum| copy.rename_part().map(|()| checksum))
        .map_err(|failure| failure.into_error(copy.remove(&part)))?;
    // From here on, the target is the import's own, unverified file; a failure
    // removes it.
    copy.sync_target_folder()
        .and_then(|()| copy.verify_target(&source_checksum))
        .map_err(|failure| failure.into_error(copy.remove(target)))?;
    Ok(source_checksum)
}

/// `<target>.part`, next to the target.
fn part_path(target: &Path) -> PathBuf {
    let mut name = OsString::from(target.as_os_str());
    name.push(".part");
    PathBuf::from(name)
}

/// A failed step before cleaning up.
struct Failure {
    step: CopyStep,
    kind: CopyErrorKind,
}

impl Failure {
    fn io(step: CopyStep, path: &Path) -> impl FnOnce(io::Error) -> Failure {
        let path = path.to_path_buf();
        move |error| Failure {
            step,
            kind: CopyErrorKind::Io { path, error },
        }
    }

    fn into_error(self, cleanup: Option<Box<CleanupFailure>>) -> CopyError {
        CopyError {
            step: self.step,
            kind: self.kind,
            cleanup,
        }
    }
}

struct Copy<'a> {
    file_system: &'a dyn FileSystem,
    hasher: &'a dyn Hasher,
    source: &'a Path,
    target: &'a Path,
    part: &'a Path,
}

impl Copy<'_> {
    /// Creates the missing folders of the target, outermost first, and syncs each
    /// into its parent so that it survives a crash with the file in it.
    fn create_missing_folders(&self) -> Result<(), Failure> {
        let mut missing = Vec::new();
        for folder in self.target.ancestors().skip(1) {
            if folder.as_os_str().is_empty() {
                break;
            }
            let entry = self
                .file_system
                .entry(folder)
                .map_err(Failure::io(CopyStep::CreateFolders, folder))?;
            if entry.is_some() {
                break;
            }
            missing.push(folder);
        }
        for folder in missing.into_iter().rev() {
            self.create_folder(folder)?;
        }
        Ok(())
    }

    fn create_folder(&self, folder: &Path) -> Result<(), Failure> {
        let fail = || Failure::io(CopyStep::CreateFolders, folder);
        match self.file_system.create_folder(folder) {
            Ok(()) => {}
            // Another program created it meanwhile; a folder is all that is needed.
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let entry = self.file_system.entry(folder).map_err(fail())?;
                if entry != Some(EntryKind::Folder) {
                    return Err(fail()(error));
                }
            }
            Err(error) => return Err(fail()(error)),
        }
        let parent = folder.parent().unwrap_or(Path::new(""));
        self.file_system
            .sync_folder(parent)
            .map_err(Failure::io(CopyStep::CreateFolders, parent))
    }

    /// Reads the source once, hashing it while writing it to the `.part` file, then
    /// timestamps and syncs the `.part` file. Returns the source's checksum.
    fn write_part(&self) -> Result<Checksum, Failure> {
        let read_failed = || Failure::io(CopyStep::ReadSource, self.source);
        let write_failed = || Failure::io(CopyStep::WritePart, self.part);
        let modified = self
            .file_system
            .modification_time(self.source)
            .map_err(read_failed())?;
        let mut reader = self
            .file_system
            .open_read(self.source)
            .map_err(read_failed())?;
        let mut part = self
            .file_system
            .create_file(self.part)
            .map_err(write_failed())?;
        let mut hash = self.hasher.start();
        let mut buffer = vec![0; BUFFER_SIZE];
        loop {
            let count = read_some(&mut *reader, &mut buffer).map_err(read_failed())?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
            part.write_all(&buffer[..count]).map_err(write_failed())?;
        }
        part.flush().map_err(write_failed())?;
        part.set_modification_time(modified)
            .map_err(write_failed())?;
        part.sync().map_err(write_failed())?;
        Ok(hash.finish())
    }

    /// Renames the `.part` file to the target without replacing a file.
    fn rename_part(&self) -> Result<(), Failure> {
        match self.file_system.rename_no_replace(self.part, self.target) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(Failure {
                step: CopyStep::Rename,
                kind: CopyErrorKind::TargetExists {
                    path: self.target.to_path_buf(),
                },
            }),
            Err(error) => Err(Failure::io(CopyStep::Rename, self.target)(error)),
        }
    }

    /// Syncs the target's folder so the new name survives a crash.
    fn sync_target_folder(&self) -> Result<(), Failure> {
        let folder = self.target.parent().unwrap_or(Path::new(""));
        self.file_system
            .sync_folder(folder)
            .map_err(Failure::io(CopyStep::SyncFolder, folder))
    }

    /// Reads the target back in full and compares its checksum with the source's.
    fn verify_target(&self, source_checksum: &Checksum) -> Result<(), Failure> {
        let target_checksum = checksum_file(self.file_system, self.hasher, self.target)
            .map_err(Failure::io(CopyStep::ReadBack, self.target))?;
        if target_checksum != *source_checksum {
            return Err(Failure {
                step: CopyStep::Verify,
                kind: CopyErrorKind::ChecksumMismatch {
                    source_checksum: source_checksum.clone(),
                    target_checksum,
                },
            });
        }
        Ok(())
    }

    /// Removes a file the copy wrote; a file that is already gone is fine.
    fn remove(&self, path: &Path) -> Option<Box<CleanupFailure>> {
        match self.file_system.remove_file(path) {
            Ok(()) => None,
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => Some(Box::new(CleanupFailure {
                path: path.to_path_buf(),
                error,
            })),
        }
    }
}
