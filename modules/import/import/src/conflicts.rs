// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Name conflicts and files already imported (IMPORT-003).

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use domain::Checksum;
use files::{EntryKind, FileSystem, Hasher, checksum_file};

/// What the import does when a target name holds a different file or a folder
/// (glossary: Conflict policy).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ConflictPolicy {
    /// Use the first free name `<base>-1.<ext>`, `<base>-2.<ext>`, … (`add_suffix`).
    #[default]
    AddSuffix,
    /// Do not copy the file (`skip`).
    Skip,
    /// Do not copy the file, and report it as failed (`fail`).
    Fail,
}

/// What conflict resolution decided for one file of the import plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictOutcome {
    /// Copy the file to `target`, the planned target or one with a suffix.
    CopyTo {
        /// The path relative to the import destination, with `/` between segments.
        target: String,
    },
    /// The destination already holds the file's content at `target`; not copied.
    AlreadyImported {
        /// The existing file, relative to the import destination.
        target: String,
    },
    /// An earlier file of the same plan has the same target and content; not copied.
    DuplicateOf {
        /// The earlier file, relative to the source.
        earlier: PathBuf,
    },
    /// The target holds a different file and the policy is `skip`; not copied.
    SkippedNameConflict {
        /// The planned target.
        target: String,
    },
    /// The target holds a different file and the policy is `fail`; not copied and
    /// reported as failed.
    FailedNameConflict {
        /// The planned target.
        target: String,
    },
}

impl fmt::Display for ConflictOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConflictOutcome::CopyTo { target } => write!(formatter, "copy to {target}"),
            ConflictOutcome::AlreadyImported { .. } => formatter.write_str("already imported"),
            ConflictOutcome::DuplicateOf { earlier } => {
                write!(formatter, "duplicate of {}", earlier.display())
            }
            ConflictOutcome::SkippedNameConflict { .. } => {
                formatter.write_str("skipped: name conflict")
            }
            ConflictOutcome::FailedNameConflict { .. } => {
                formatter.write_str("failed: name conflict")
            }
        }
    }
}

/// A file or folder that could not be inspected while a target was checked.
#[derive(Debug, thiserror::Error)]
#[error("cannot check the target: {} cannot be read: {source}", path.display())]
pub struct ConflictError {
    /// The file or folder that could not be inspected.
    pub path: PathBuf,
    /// The error of the file system.
    #[source]
    pub source: io::Error,
}

/// One file to check, in scan order.
#[derive(Debug, Clone, Copy)]
pub struct TargetRequest<'a> {
    /// The file, relative to the source.
    pub source_path: &'a Path,
    /// Its size in bytes, from the scan.
    pub size: u64,
    /// The planned target, relative to the destination, with `/` between segments.
    pub target: &'a str,
}

/// Resolves the targets of an import plan, file by file in scan order, against the
/// destination and against the earlier files of the plan (IMPORT-003).
///
/// It never writes. Two targets that differ only in case are the same target; the
/// names are compared after [`str::to_lowercase`]. Content is compared by size first,
/// and by checksum only when the sizes are equal.
pub struct TargetResolver<'a> {
    file_system: &'a dyn FileSystem,
    hasher: &'a dyn Hasher,
    source: &'a Path,
    destination: &'a Path,
    policy: ConflictPolicy,
    /// Targets given to earlier files, by lower-case target.
    claimed: HashMap<String, Claim>,
    /// The listing of each target folder, by lower-case name; read once.
    folders: HashMap<String, HashMap<String, Existing>>,
    /// Checksums by absolute path, so that no file is read twice.
    checksums: HashMap<PathBuf, Checksum>,
}

#[derive(Debug, Clone)]
struct Claim {
    source_path: PathBuf,
    size: u64,
}

#[derive(Debug, Clone)]
struct Existing {
    name: String,
    kind: EntryKind,
}

/// What a candidate name holds.
enum Holder {
    Free,
    Same(ConflictOutcome),
    Different,
}

impl<'a> TargetResolver<'a> {
    /// Creates a resolver for files of `source` copied to `destination`.
    #[must_use]
    pub fn new(
        file_system: &'a dyn FileSystem,
        hasher: &'a dyn Hasher,
        source: &'a Path,
        destination: &'a Path,
        policy: ConflictPolicy,
    ) -> Self {
        TargetResolver {
            file_system,
            hasher,
            source,
            destination,
            policy,
            claimed: HashMap::new(),
            folders: HashMap::new(),
            checksums: HashMap::new(),
        }
    }

    /// Decides what happens to `file`. A target it is given counts as taken for the
    /// later files.
    ///
    /// # Errors
    ///
    /// Returns a [`ConflictError`] if the target folder or a file to compare cannot
    /// be read.
    pub fn resolve(&mut self, file: &TargetRequest<'_>) -> Result<ConflictOutcome, ConflictError> {
        for candidate in candidates(file.target) {
            match self.holder(file, &candidate)? {
                Holder::Free => {
                    self.claim(file, &candidate);
                    return Ok(ConflictOutcome::CopyTo { target: candidate });
                }
                Holder::Same(outcome) => return Ok(outcome),
                Holder::Different => {}
            }
            let target = file.target.to_owned();
            match self.policy {
                ConflictPolicy::AddSuffix => {}
                ConflictPolicy::Skip => return Ok(ConflictOutcome::SkippedNameConflict { target }),
                ConflictPolicy::Fail => return Ok(ConflictOutcome::FailedNameConflict { target }),
            }
        }
        unreachable_candidates()
    }

    fn claim(&mut self, file: &TargetRequest<'_>, candidate: &str) {
        let claim = Claim {
            source_path: file.source_path.to_path_buf(),
            size: file.size,
        };
        self.claimed.insert(candidate.to_lowercase(), claim);
    }

    fn holder(
        &mut self,
        file: &TargetRequest<'_>,
        candidate: &str,
    ) -> Result<Holder, ConflictError> {
        if let Some(claim) = self.claimed.get(&candidate.to_lowercase()).cloned() {
            let earlier = self.source.join(&claim.source_path);
            return Ok(if self.same_content(file, claim.size, &earlier)? {
                Holder::Same(ConflictOutcome::DuplicateOf {
                    earlier: claim.source_path,
                })
            } else {
                Holder::Different
            });
        }
        let (folder, name) = split_target(candidate);
        let Some(existing) = self.existing(folder, name)? else {
            return Ok(Holder::Free);
        };
        if existing.kind != EntryKind::File {
            return Ok(Holder::Different);
        }
        let target = join_target(folder, &existing.name);
        let path = self.destination.join(&target);
        let size = self
            .file_system
            .file_size(&path)
            .map_err(cannot_read(&path))?;
        Ok(if self.same_content(file, size, &path)? {
            Holder::Same(ConflictOutcome::AlreadyImported { target })
        } else {
            Holder::Different
        })
    }

    /// Whether `file` has the same content as the file `other` of `other_size` bytes.
    fn same_content(
        &mut self,
        file: &TargetRequest<'_>,
        other_size: u64,
        other: &Path,
    ) -> Result<bool, ConflictError> {
        if file.size != other_size {
            return Ok(false);
        }
        let ours = self.checksum(&self.source.join(file.source_path))?;
        let theirs = self.checksum(other)?;
        Ok(ours == theirs)
    }

    fn checksum(&mut self, path: &Path) -> Result<Checksum, ConflictError> {
        if let Some(checksum) = self.checksums.get(path) {
            return Ok(checksum.clone());
        }
        let checksum =
            checksum_file(self.file_system, self.hasher, path).map_err(cannot_read(path))?;
        self.checksums.insert(path.to_path_buf(), checksum.clone());
        Ok(checksum)
    }

    /// The entry at the destination whose name equals `name`, ignoring case.
    fn existing(&mut self, folder: &str, name: &str) -> Result<Option<Existing>, ConflictError> {
        let key = folder.to_lowercase();
        if !self.folders.contains_key(&key) {
            let listing = self.list_target_folder(folder)?;
            self.folders.insert(key.clone(), listing);
        }
        Ok(self
            .folders
            .get(&key)
            .and_then(|listing| listing.get(&name.to_lowercase()))
            .cloned())
    }

    fn list_target_folder(&self, folder: &str) -> Result<HashMap<String, Existing>, ConflictError> {
        let path = self.destination.join(folder);
        let entries = match self.file_system.list_folder(&path) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(cannot_read(&path)(error)),
        };
        Ok(entries
            .into_iter()
            .map(|entry| {
                let name = entry.name.to_string_lossy().into_owned();
                (
                    name.to_lowercase(),
                    Existing {
                        name,
                        kind: entry.kind,
                    },
                )
            })
            .collect())
    }
}

/// The planned target, then `<base>-1.<ext>`, `<base>-2.<ext>`, …
fn candidates(target: &str) -> impl Iterator<Item = String> + '_ {
    let (folder, name) = split_target(target);
    let (base, extension) = match name.rsplit_once('.') {
        Some((base, extension)) => (base, format!(".{extension}")),
        None => (name, String::new()),
    };
    let suffixed =
        (1_u64..).map(move |number| join_target(folder, &format!("{base}-{number}{extension}")));
    std::iter::once(target.to_owned()).chain(suffixed)
}

/// The suffixes never run out: a folder holds finitely many names.
fn unreachable_candidates() -> Result<ConflictOutcome, ConflictError> {
    Err(ConflictError {
        path: PathBuf::new(),
        source: io::Error::other("no free name with a suffix"),
    })
}

/// The folder part and the name of a target; the folder is empty at the top.
fn split_target(target: &str) -> (&str, &str) {
    target.rsplit_once('/').unwrap_or(("", target))
}

fn join_target(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_owned()
    } else {
        format!("{folder}/{name}")
    }
}

fn cannot_read(path: &Path) -> impl Fn(io::Error) -> ConflictError + '_ {
    move |source| ConflictError {
        path: path.to_path_buf(),
        source,
    }
}
