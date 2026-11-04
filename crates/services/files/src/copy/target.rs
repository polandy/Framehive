// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! One target of a verified copy: its `.part` file, rename and verification.

use std::io;
use std::path::{Path, PathBuf};

use domain::{Checksum, Timestamp};

use super::{Copy, Failure, part_path};
use crate::checksum::checksum_file;
use crate::{CleanupFailure, CopyError, CopyErrorKind, CopyStep, EntryKind, FileSystem, WriteFile};

/// Where a target of a verified copy is.
enum State {
    /// Not started, or its folders are created.
    Open,
    /// Its `.part` file is being written.
    Writing(Box<dyn WriteFile>),
    /// Verified, with the source's checksum.
    Verified(Checksum),
    /// Failed; its `.part` file and unverified target are removed, if possible.
    Failed(CopyError),
}

/// One target of a verified copy.
pub(super) struct TargetCopy<'a> {
    target: &'a Path,
    part: PathBuf,
    state: State,
}

impl<'a> TargetCopy<'a> {
    pub(super) fn new(target: &'a Path) -> Self {
        TargetCopy {
            target,
            part: part_path(target),
            state: State::Open,
        }
    }

    /// Whether the target has not failed and is not verified yet.
    pub(super) fn is_open(&self) -> bool {
        matches!(self.state, State::Open | State::Writing(_))
    }

    /// The target's result, once the copy is done.
    pub(super) fn into_result(self) -> Result<Checksum, CopyError> {
        match self.state {
            State::Verified(checksum) => Ok(checksum),
            State::Failed(error) => Err(error),
            // Never expected: the copy leaves each target verified or failed;
            // reported instead of panicking.
            State::Open | State::Writing(_) => Err(CopyError {
                step: CopyStep::Verify,
                kind: CopyErrorKind::Io {
                    path: self.target.to_path_buf(),
                    error: io::Error::other("the copy did not finish"),
                },
                cleanup: None,
            }),
        }
    }

    /// Creates the missing folders of the target; the target fails if one cannot be
    /// created.
    pub(super) fn prepare_folders(&mut self, file_system: &dyn FileSystem) {
        if let Err(failure) = self.create_missing_folders(file_system) {
            self.state = State::Failed(failure.into_error(None));
        }
    }

    /// Creates the missing folders of the target, outermost first, and syncs each
    /// into its parent so that it survives a crash with the file in it.
    fn create_missing_folders(&self, file_system: &dyn FileSystem) -> Result<(), Failure> {
        let mut missing = Vec::new();
        for folder in self.target.ancestors().skip(1) {
            if folder.as_os_str().is_empty() {
                break;
            }
            let entry = file_system
                .entry(folder)
                .map_err(Failure::io(CopyStep::CreateFolders, folder))?;
            if entry.is_some() {
                break;
            }
            missing.push(folder);
        }
        for folder in missing.into_iter().rev() {
            create_folder(file_system, folder)?;
        }
        Ok(())
    }

    /// Creates the `.part` file, which replaces one left by an interrupted import.
    pub(super) fn create_part(&mut self, file_system: &dyn FileSystem) {
        if !matches!(self.state, State::Open) {
            return;
        }
        match file_system.create_file(&self.part) {
            Ok(writer) => self.state = State::Writing(writer),
            Err(error) => {
                let failure = Failure::io(CopyStep::WritePart, &self.part)(error);
                self.fail_before_rename(file_system, failure);
            }
        }
    }

    /// Writes `bytes` to the `.part` file; a failure drops the target from the copy.
    pub(super) fn write(&mut self, file_system: &dyn FileSystem, bytes: &[u8]) {
        let State::Writing(writer) = &mut self.state else {
            return;
        };
        if let Err(error) = writer.write_all(bytes) {
            let failure = Failure::io(CopyStep::WritePart, &self.part)(error);
            self.fail_before_rename(file_system, failure);
        }
    }

    /// Timestamps, syncs and renames the `.part` file, then syncs the folder and
    /// verifies the target against `source_checksum`.
    pub(super) fn finish(
        &mut self,
        copy: &Copy<'_>,
        modified: Timestamp,
        source_checksum: &Checksum,
    ) {
        if !matches!(self.state, State::Writing(_)) {
            return;
        }
        let State::Writing(writer) = std::mem::replace(&mut self.state, State::Open) else {
            return;
        };
        let file_system = copy.file_system;
        if let Err(failure) =
            finish_part(writer, &self.part, modified).and_then(|()| self.rename_part(file_system))
        {
            self.fail_before_rename(file_system, failure);
            return;
        }
        // From here on, the target is the import's own, unverified file; a failure
        // removes it.
        match self
            .sync_target_folder(file_system)
            .and_then(|()| self.verify_target(copy, source_checksum))
        {
            Ok(()) => self.state = State::Verified(source_checksum.clone()),
            Err(failure) => {
                let cleanup = remove(file_system, self.target);
                self.state = State::Failed(failure.into_error(cleanup));
            }
        }
    }

    /// Fails the target before its `.part` file is renamed: closes and removes it.
    pub(super) fn fail_before_rename(&mut self, file_system: &dyn FileSystem, failure: Failure) {
        // Closes the `.part` file before it is removed.
        self.state = State::Open;
        let cleanup = remove(file_system, &self.part);
        self.state = State::Failed(failure.into_error(cleanup));
    }

    /// Renames the `.part` file to the target without replacing a file.
    fn rename_part(&self, file_system: &dyn FileSystem) -> Result<(), Failure> {
        match file_system.rename_no_replace(&self.part, self.target) {
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
    fn sync_target_folder(&self, file_system: &dyn FileSystem) -> Result<(), Failure> {
        let folder = self.target.parent().unwrap_or(Path::new(""));
        file_system
            .sync_folder(folder)
            .map_err(Failure::io(CopyStep::SyncFolder, folder))
    }

    /// Reads the target back in full and compares its checksum with the source's.
    fn verify_target(&self, copy: &Copy<'_>, source_checksum: &Checksum) -> Result<(), Failure> {
        let target_checksum = checksum_file(copy.file_system, copy.hasher, self.target)
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
}

fn create_folder(file_system: &dyn FileSystem, folder: &Path) -> Result<(), Failure> {
    let fail = || Failure::io(CopyStep::CreateFolders, folder);
    match file_system.create_folder(folder) {
        Ok(()) => {}
        // Another program created it meanwhile; a folder is all that is needed.
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let entry = file_system.entry(folder).map_err(fail())?;
            if entry != Some(EntryKind::Folder) {
                return Err(fail()(error));
            }
        }
        Err(error) => return Err(fail()(error)),
    }
    let parent = folder.parent().unwrap_or(Path::new(""));
    file_system
        .sync_folder(parent)
        .map_err(Failure::io(CopyStep::CreateFolders, parent))
}

/// Flushes, timestamps and syncs a `.part` file, and closes it.
fn finish_part(
    mut writer: Box<dyn WriteFile>,
    part: &Path,
    modified: Timestamp,
) -> Result<(), Failure> {
    let write_failed = || Failure::io(CopyStep::WritePart, part);
    writer.flush().map_err(write_failed())?;
    writer
        .set_modification_time(modified)
        .map_err(write_failed())?;
    writer.sync().map_err(write_failed())
}

/// Removes a file the copy wrote; a file that is already gone is fine.
fn remove(file_system: &dyn FileSystem, path: &Path) -> Option<Box<CleanupFailure>> {
    match file_system.remove_file(path) {
        Ok(()) => None,
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => Some(Box::new(CleanupFailure {
            path: path.to_path_buf(),
            error,
        })),
    }
}
