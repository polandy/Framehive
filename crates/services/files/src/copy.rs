// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The verified copy of one file (IMPORT-004, ADR 0020 §3).

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use domain::Checksum;

use crate::checksum::{BUFFER_SIZE, read_some};
use crate::{CleanupFailure, CopyError, CopyErrorKind, CopyStep, FileSystem, Hasher};

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
    let copy = Copy {
        file_system,
        hasher,
        source,
    };
    let mut targets = [TargetCopy::new(target)];
    copy.copy_to(&mut targets);
    let [target] = targets;
    target.into_result()
}

/// The result of [`copy_verified_to_all`].
#[derive(Debug)]
pub struct VerifiedCopies {
    /// The source's checksum, when the source was read to its end; `None` when it
    /// could not be read or every target failed before the end.
    pub source_checksum: Option<Checksum>,
    /// One result per target, in the order of the targets: the source's checksum
    /// once that target is verified, or why it failed.
    pub targets: Vec<Result<Checksum, CopyError>>,
}

/// Copies `source` to each of `targets` and verifies each copy, reading the source
/// once (IMPORT-007, ADR 0030 §1).
///
/// The source is read once; its bytes are hashed and written to the `.part` file of
/// every target. Each target then goes through the steps of [`copy_verified`] on its
/// own and is compared with the source's checksum. A failure at one target removes
/// only that target's `.part` file or unverified target; the other targets go on. A
/// target whose `.part` file cannot be written is dropped from the rest of the read,
/// and the read ends early only when every target failed. With no targets, nothing
/// is read.
///
/// [`copy_verified`] is this copy with one target.
///
/// ```
/// use std::path::Path;
///
/// use domain::Timestamp;
/// use files::copy_verified_to_all;
/// use files::testing::{FnvHasher, MemoryFileSystem};
///
/// let file_system = MemoryFileSystem::new();
/// file_system.add_file(Path::new("/card/DSCF0001.RAF"), b"raw", Timestamp::UNIX_EPOCH);
/// let targets = [Path::new("/photos/DSCF0001.RAF"), Path::new("/backup/DSCF0001.RAF")];
///
/// let copies = copy_verified_to_all(&file_system, &FnvHasher, Path::new("/card/DSCF0001.RAF"), &targets);
///
/// assert!(copies.targets.iter().all(Result::is_ok));
/// assert_eq!(file_system.content(targets[1]).as_deref(), Some(&b"raw"[..]));
/// ```
#[must_use]
pub fn copy_verified_to_all(
    file_system: &dyn FileSystem,
    hasher: &dyn Hasher,
    source: &Path,
    targets: &[&Path],
) -> VerifiedCopies {
    let copy = Copy {
        file_system,
        hasher,
        source,
    };
    let mut copies: Vec<TargetCopy<'_>> = targets
        .iter()
        .map(|target| TargetCopy::new(target))
        .collect();
    let source_checksum = copy.copy_to(&mut copies);
    let targets = copies.into_iter().map(TargetCopy::into_result).collect();
    VerifiedCopies {
        source_checksum,
        targets,
    }
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

/// The source of a verified copy and the ports it goes through.
struct Copy<'a> {
    file_system: &'a dyn FileSystem,
    hasher: &'a dyn Hasher,
    source: &'a Path,
}

impl Copy<'_> {
    /// Copies the source to every target and verifies each, reading the source
    /// once. Leaves every target verified or failed, and returns the source's
    /// checksum if the source was read to its end.
    fn copy_to(&self, targets: &mut [TargetCopy<'_>]) -> Option<Checksum> {
        for target in targets.iter_mut() {
            target.prepare_folders(self.file_system);
        }
        if !targets.iter().any(TargetCopy::is_open) {
            return None;
        }
        let modified = match self.file_system.modification_time(self.source) {
            Ok(modified) => modified,
            Err(error) => return self.fail_reading(targets, error),
        };
        let mut reader = match self.file_system.open_read(self.source) {
            Ok(reader) => reader,
            Err(error) => return self.fail_reading(targets, error),
        };
        for target in targets.iter_mut() {
            target.create_part(self.file_system);
        }
        let source_checksum = self.read_into_parts(&mut *reader, targets)?;
        for target in targets.iter_mut() {
            target.finish(self, modified, &source_checksum);
        }
        Some(source_checksum)
    }

    /// Reads the source once, hashing it while writing it to the `.part` file of each
    /// target that has not failed. Returns the source's checksum, or `None` when the
    /// source could not be read or every target failed before its end.
    fn read_into_parts(
        &self,
        reader: &mut dyn io::Read,
        targets: &mut [TargetCopy<'_>],
    ) -> Option<Checksum> {
        let mut hash = self.hasher.start();
        let mut buffer = vec![0; BUFFER_SIZE];
        while targets.iter().any(TargetCopy::is_open) {
            let count = match read_some(reader, &mut buffer) {
                Ok(count) => count,
                Err(error) => return self.fail_reading(targets, error),
            };
            if count == 0 {
                return Some(hash.finish());
            }
            hash.update(&buffer[..count]);
            for target in targets.iter_mut() {
                target.write(self.file_system, &buffer[..count]);
            }
        }
        None
    }

    /// Fails every target that has not failed yet, because the source cannot be read.
    /// Only the first gets `error` itself; the others get one of the same kind and
    /// message.
    fn fail_reading(&self, targets: &mut [TargetCopy<'_>], error: io::Error) -> Option<Checksum> {
        let (kind, message) = (error.kind(), error.to_string());
        let mut error = Some(error);
        for target in targets.iter_mut().filter(|target| target.is_open()) {
            let error = error
                .take()
                .unwrap_or_else(|| io::Error::new(kind, message.clone()));
            target.fail_before_rename(
                self.file_system,
                Failure::io(CopyStep::ReadSource, self.source)(error),
            );
        }
        None
    }
}

mod target;

use target::TargetCopy;
