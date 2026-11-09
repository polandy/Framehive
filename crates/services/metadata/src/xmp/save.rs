// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading and saving sidecar files through the `files` port (CORE-010, saving;
//! ADR 0040 §7).

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use domain::{Checksum, Clock, Timestamp};
use files::{EntryKind, FileSystem, Hasher, checksum_file};

use super::error::{SidecarError, SidecarErrorKind, SidecarStep, XmpEditError};
use super::model::XmpPacket;
use super::parse::parse_xmp;
use super::serialize::serialize_xmp;
use super::xml::XmlSyntax;

/// What was at a sidecar's path when it was read. A write compares it with what is
/// there before its rename, so that it never replaces another program's change
/// (CORE-010.3, .4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundSidecar(Found);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Found {
    Nothing,
    File {
        size: u64,
        modified: Timestamp,
        checksum: Checksum,
    },
    NotAFile,
}

impl FoundSidecar {
    /// Nothing at the path, for a sidecar that is written new, such as at an import
    /// target whose name conflict resolution found free (IMPORT-019).
    #[must_use]
    pub fn nothing() -> Self {
        FoundSidecar(Found::Nothing)
    }

    /// Whether nothing was at the path.
    #[must_use]
    pub fn is_nothing(&self) -> bool {
        self.0 == Found::Nothing
    }
}

/// A sidecar as read: its packet, empty when there was none, and what was found.
#[derive(Debug, Clone)]
pub struct ReadSidecar {
    /// The parsed packet.
    pub packet: XmpPacket,
    /// What was at the path.
    pub found: FoundSidecar,
}

/// Reads, merges and saves sidecars through the ports.
#[derive(Clone, Copy)]
pub struct SidecarFiles<'a> {
    /// Where the sidecars are.
    pub files: &'a dyn FileSystem,
    /// The hash that tells whether a sidecar changed, BLAKE3 in the apps.
    pub hasher: &'a dyn Hasher,
    /// The XML tokenizer.
    pub xml: &'a dyn XmlSyntax,
    /// The time of `xmp:MetadataDate`.
    pub clock: &'a dyn Clock,
}

impl SidecarFiles<'_> {
    /// Reads the sidecar at `path`; an empty packet when nothing is there.
    ///
    /// # Errors
    ///
    /// A [`SidecarError`] naming `path` when something other than a file is there,
    /// or the file cannot be read or parsed.
    pub fn read(&self, path: &Path) -> Result<ReadSidecar, SidecarError> {
        let fail = |kind| SidecarError::new(path, kind);
        let entry = self
            .files
            .entry(path)
            .map_err(io_error(path, SidecarStep::Read))?;
        match entry {
            None => {
                return Ok(ReadSidecar {
                    packet: XmpPacket::new(),
                    found: FoundSidecar(Found::Nothing),
                });
            }
            Some(EntryKind::File) => {}
            Some(_) => return Err(fail(SidecarErrorKind::NotAFile)),
        }
        let (modified, content) = self
            .read_content(path)
            .map_err(io_error(path, SidecarStep::Read))?;
        let packet = parse_xmp(path, &content, self.xml)
            .map_err(|error| fail(SidecarErrorKind::Unreadable(error.reason)))?;
        let mut hash = self.hasher.start();
        hash.update(&content);
        let found = Found::File {
            size: u64::try_from(content.len()).unwrap_or(u64::MAX),
            modified,
            checksum: hash.finish(),
        };
        Ok(ReadSidecar {
            packet,
            found: FoundSidecar(found),
        })
    }

    /// Reads the sidecar at `path`, applies `change` and saves it with
    /// [`SidecarFiles::write`]. A sidecar that cannot be parsed is never overwritten.
    ///
    /// # Errors
    ///
    /// A [`SidecarError`] naming `path` when reading, the change, serializing or
    /// writing fails.
    pub fn merge(
        &self,
        path: &Path,
        change: impl FnOnce(&mut XmpPacket) -> Result<(), XmpEditError>,
    ) -> Result<(), SidecarError> {
        let mut sidecar = self.read(path)?;
        change(&mut sidecar.packet)
            .map_err(|error| SidecarError::new(path, SidecarErrorKind::Edit(error)))?;
        let content = serialize_xmp(&sidecar.packet, self.clock, self.xml)
            .map_err(|error| SidecarError::new(path, SidecarErrorKind::Clock(error)))?;
        self.write(path, content.as_bytes(), &sidecar.found)
    }

    /// Writes `content` to `<path>.part` and renames it to `path`, unless what is at
    /// `path` differs from `found`. A failed write leaves `path` as it was and
    /// removes the `.part` file.
    ///
    /// # Errors
    ///
    /// A [`SidecarError`] naming `path` when a sidecar appeared or changed since
    /// `found`, or a step of the write fails.
    pub fn write(
        &self,
        path: &Path,
        content: &[u8],
        found: &FoundSidecar,
    ) -> Result<(), SidecarError> {
        let part = part_path(path);
        let written = self
            .write_part(&part, content)
            .map_err(io_error(path, SidecarStep::WritePart))
            .and_then(|()| self.check_unchanged(path, found))
            .and_then(|()| self.rename(&part, path, found));
        match written {
            Ok(()) => self.sync_folder(path),
            Err(mut error) => {
                if self.files.entry(&part).ok().flatten().is_some()
                    && self.files.remove_file(&part).is_err()
                {
                    error.part_left = Some(part);
                }
                Err(error)
            }
        }
    }

    fn read_content(&self, path: &Path) -> io::Result<(Timestamp, Vec<u8>)> {
        let modified = self.files.modification_time(path)?;
        let mut content = Vec::new();
        self.files.open_read(path)?.read_to_end(&mut content)?;
        Ok((modified, content))
    }

    fn write_part(&self, part: &Path, content: &[u8]) -> io::Result<()> {
        let mut file = self.files.create_file(part)?;
        file.write_all(content)?;
        file.sync()
    }

    fn check_unchanged(&self, path: &Path, found: &FoundSidecar) -> Result<(), SidecarError> {
        let now = self
            .found_now(path)
            .map_err(io_error(path, SidecarStep::Check))?;
        if now == found.0 {
            return Ok(());
        }
        let kind = if found.is_nothing() {
            SidecarErrorKind::Appeared
        } else {
            SidecarErrorKind::Changed
        };
        Err(SidecarError::new(path, kind))
    }

    fn found_now(&self, path: &Path) -> io::Result<Found> {
        Ok(match self.files.entry(path)? {
            None => Found::Nothing,
            Some(EntryKind::File) => Found::File {
                size: self.files.file_size(path)?,
                modified: self.files.modification_time(path)?,
                checksum: checksum_file(self.files, self.hasher, path)?,
            },
            Some(_) => Found::NotAFile,
        })
    }

    fn rename(&self, part: &Path, path: &Path, found: &FoundSidecar) -> Result<(), SidecarError> {
        let renamed = if found.is_nothing() {
            self.files.rename_no_replace(part, path)
        } else {
            self.files.replace_file(part, path)
        };
        renamed.map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists if found.is_nothing() => {
                SidecarError::new(path, SidecarErrorKind::Appeared)
            }
            _ => io_error(path, SidecarStep::Rename)(error),
        })
    }

    fn sync_folder(&self, path: &Path) -> Result<(), SidecarError> {
        match path.parent() {
            Some(folder) if !folder.as_os_str().is_empty() => self
                .files
                .sync_folder(folder)
                .map_err(io_error(path, SidecarStep::SyncFolder)),
            _ => Ok(()),
        }
    }
}

fn part_path(path: &Path) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(".part");
    PathBuf::from(name)
}

fn io_error(path: &Path, step: SidecarStep) -> impl Fn(io::Error) -> SidecarError + '_ {
    move |source| SidecarError::new(path, SidecarErrorKind::Io { step, source })
}
