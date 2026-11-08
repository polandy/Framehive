// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Name conflicts and files already imported (IMPORT-003).

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use domain::Checksum;
use files::{EntryKind, FileSystem, Hasher, ReadOnlyFileSystem, checksum_file};
use metadata::xmp::SidecarNaming;

mod folders;
mod reserved;
mod sidecars;
mod target_names;

use folders::Folders;
pub(crate) use sidecars::sidecar_target;
pub(crate) use target_names::caseless_key;
use target_names::{candidates, split_target};

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
    /// A folder of the target path is a file at the destination, so the file cannot
    /// be copied under any policy (IMPORT-017); reported as failed.
    TargetFolderIsAFile {
        /// The planned target.
        target: String,
        /// The file in the way, spelled as found at the destination.
        folder: PathBuf,
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
            ConflictOutcome::TargetFolderIsAFile { folder, .. } => {
                write!(
                    formatter,
                    "failed: the target folder {} is a file",
                    folder.display()
                )
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
    /// How its XMP sidecar is named, when it gets one: the sidecar's name must be
    /// free with the target's (IMPORT-019, "Conflicts").
    pub sidecar: Option<SidecarNaming>,
}

/// Resolves the targets of an import plan, file by file in scan order, against the
/// destination and against the earlier files of the plan (IMPORT-003).
///
/// It never writes. Two targets that differ only in case or in their Unicode
/// normalization form are the same target; the names are compared by their
/// canonical caseless key (ADR 0026). Content is compared by size first, and by
/// checksum only when the sizes are equal.
pub struct TargetResolver<'a> {
    /// Reads the source (ADR 0034).
    source_file_system: &'a dyn ReadOnlyFileSystem,
    /// Reads the destination.
    file_system: &'a dyn FileSystem,
    hasher: &'a dyn Hasher,
    source: &'a Path,
    destination: &'a Path,
    policy: ConflictPolicy,
    /// Targets given to earlier files, by the canonical caseless key of the target.
    claimed: HashMap<String, Claim>,
    /// Sidecar names given to earlier files, by canonical caseless key (IMPORT-019).
    claimed_sidecars: HashSet<String>,
    /// Folders another destination of the run needs, by canonical caseless key, so
    /// that no file takes their names (IMPORT-017).
    needed_folders: HashSet<String>,
    /// The listing of each folder at the destination, by its path as found there,
    /// with the entries by canonical caseless key; read once. A destination that
    /// tells case or normalization forms apart may hold several entries under one
    /// key, and each of them may hold the content.
    folders: HashMap<String, HashMap<String, Vec<Existing>>>,
    /// Checksums of source files by absolute path, so that no file is read twice.
    source_checksums: HashMap<PathBuf, Checksum>,
    /// Checksums of destination files by absolute path, so that no file is read twice.
    checksums: HashMap<PathBuf, Checksum>,
    /// The target names other import jobs reserved (IMPORT-015).
    reserved: Option<reserved::Reserved<'a>>,
}

#[derive(Debug, Clone)]
struct Claim {
    source_path: PathBuf,
    size: u64,
}

#[derive(Debug, Clone)]
struct Existing {
    /// The path relative to the destination, spelled as found there.
    target: String,
    kind: EntryKind,
}

/// What a candidate name holds.
enum Holder {
    Free,
    Same(ConflictOutcome),
    Different,
}

impl<'a> TargetResolver<'a> {
    /// Creates a resolver for files of `source`, read through `source_file_system`,
    /// copied to `destination`, read through `file_system`.
    #[must_use]
    pub fn new(
        source_file_system: &'a dyn ReadOnlyFileSystem,
        file_system: &'a dyn FileSystem,
        hasher: &'a dyn Hasher,
        source: &'a Path,
        destination: &'a Path,
        policy: ConflictPolicy,
    ) -> Self {
        TargetResolver {
            source_file_system,
            file_system,
            hasher,
            source,
            destination,
            policy,
            claimed: HashMap::new(),
            claimed_sidecars: HashSet::new(),
            needed_folders: HashSet::new(),
            folders: HashMap::new(),
            source_checksums: HashMap::new(),
            checksums: HashMap::new(),
            reserved: None,
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
        let (folder, _) = split_target(file.target);
        if let Folders::FileInTheWay(in_the_way) = self.matching_folders(folder)? {
            return Ok(ConflictOutcome::TargetFolderIsAFile {
                target: file.target.to_owned(),
                folder: self.destination.join(in_the_way),
            });
        }
        for candidate in candidates(file.target) {
            match self.holder(file, &candidate)? {
                Holder::Free if self.sidecar_is_free(file, &candidate)? => {
                    self.claim(file, &candidate);
                    return Ok(ConflictOutcome::CopyTo { target: candidate });
                }
                Holder::Same(outcome) => return Ok(outcome),
                Holder::Free | Holder::Different => {}
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

    /// The checksum of the source file `source_path`, relative to the source; read
    /// at most once for the whole resolver, also when conflict resolution read it.
    pub(crate) fn source_checksum(
        &mut self,
        source_path: &Path,
    ) -> Result<Checksum, ConflictError> {
        let path = self.source.join(source_path);
        cached_checksum(
            &mut self.source_checksums,
            self.source_file_system,
            self.hasher,
            &path,
        )
    }

    fn claim(&mut self, file: &TargetRequest<'_>, candidate: &str) {
        let claim = Claim {
            source_path: file.source_path.to_path_buf(),
            size: file.size,
        };
        self.claimed.insert(caseless_key(candidate), claim);
        self.claim_sidecar(file, candidate);
    }

    fn holder(
        &mut self,
        file: &TargetRequest<'_>,
        candidate: &str,
    ) -> Result<Holder, ConflictError> {
        if let Some(claim) = self.claimed.get(&caseless_key(candidate)).cloned() {
            let earlier = self.source.join(&claim.source_path);
            return Ok(if self.same_content(file, claim.size, &earlier)? {
                Holder::Same(ConflictOutcome::DuplicateOf {
                    earlier: claim.source_path,
                })
            } else {
                Holder::Different
            });
        }
        if self.needed_folders.contains(&caseless_key(candidate))
            || self.claimed_for_a_sidecar(candidate)
            || self.reserved_by_another_job(candidate)
        {
            return Ok(Holder::Different);
        }
        let entries = self.existing(candidate)?;
        if entries.is_empty() {
            return Ok(Holder::Free);
        }
        for entry in entries
            .into_iter()
            .filter(|entry| entry.kind == EntryKind::File)
        {
            if self.same_as_destination_file(file, &entry.target)? {
                return Ok(Holder::Same(ConflictOutcome::AlreadyImported {
                    target: entry.target,
                }));
            }
        }
        Ok(Holder::Different)
    }

    /// Whether `file` has the same content as the file at `target` in the destination.
    fn same_as_destination_file(
        &mut self,
        file: &TargetRequest<'_>,
        target: &str,
    ) -> Result<bool, ConflictError> {
        let path = self.destination.join(target);
        let size = self
            .file_system
            .file_size(&path)
            .map_err(cannot_read(&path))?;
        self.same_content(file, size, &path)
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
        let ours = self.source_checksum(file.source_path)?;
        let theirs = cached_checksum(&mut self.checksums, self.file_system, self.hasher, other)?;
        Ok(ours == theirs)
    }

    /// The entries at the destination whose paths equal `target`, ignoring case and
    /// normalization form in every segment: the one spelled exactly like `target`
    /// first, then by path, so the outcome does not depend on the order of listings.
    fn existing(&mut self, target: &str) -> Result<Vec<Existing>, ConflictError> {
        let (folder, name) = split_target(target);
        let key = caseless_key(name);
        let mut entries = Vec::new();
        let Folders::Found(folders) = self.matching_folders(folder)? else {
            return Ok(Vec::new());
        };
        for found in folders {
            if let Some(matches) = self.listing(&found)?.get(&key) {
                entries.extend(matches.iter().cloned());
            }
        }
        entries
            .sort_by(|a, b| (a.target != target, &a.target).cmp(&(b.target != target, &b.target)));
        Ok(entries)
    }
}

/// The suffixes never run out: a folder holds finitely many names.
fn unreachable_candidates() -> Result<ConflictOutcome, ConflictError> {
    Err(ConflictError {
        path: PathBuf::new(),
        source: io::Error::other("no free name with a suffix"),
    })
}

/// The checksum of `path` from `cache`, or read through `file_system` and kept there.
fn cached_checksum(
    cache: &mut HashMap<PathBuf, Checksum>,
    file_system: &dyn ReadOnlyFileSystem,
    hasher: &dyn Hasher,
    path: &Path,
) -> Result<Checksum, ConflictError> {
    if let Some(checksum) = cache.get(path) {
        return Ok(checksum.clone());
    }
    let checksum = checksum_file(file_system, hasher, path).map_err(cannot_read(path))?;
    cache.insert(path.to_path_buf(), checksum.clone());
    Ok(checksum)
}

fn cannot_read(path: &Path) -> impl Fn(io::Error) -> ConflictError + '_ {
    move |source| ConflictError {
        path: path.to_path_buf(),
        source,
    }
}
