// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The source scan: which files of a source folder the import takes, and which it
//! leaves out and why (IMPORT-005).

use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};

use domain::{AssetFileRole, Timestamp};
use files::{EntryKind, FileSystem, FolderEntry};

use crate::roles::role_for_extension;

/// A file the scan found, to be imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundFile {
    /// The path relative to the source, e.g. `DCIM/100_FUJI/DSCF0001.RAF`.
    pub path: PathBuf,
    /// The file's role, from its extension.
    pub role: AssetFileRole,
    /// The size in bytes.
    pub size: u64,
    /// The file modification time.
    pub modification_time: Timestamp,
}

/// Why the scan left a file or folder out (IMPORT-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeftOutReason {
    /// The extension is not a supported format, or there is none.
    UnsupportedType,
    /// The name starts with `.`; a hidden folder is not entered.
    Hidden,
    /// A symbolic link, which is never followed.
    SymbolicLink,
    /// A sidecar, which would land apart from its photo until grouping (LIBRARY-001)
    /// is implemented.
    SidecarNeedsGrouping,
    /// The file or folder cannot be read; the text is the error of the file system.
    CannotBeRead(String),
}

impl fmt::Display for LeftOutReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LeftOutReason::UnsupportedType => formatter.write_str("unsupported type"),
            LeftOutReason::Hidden => formatter.write_str("hidden"),
            LeftOutReason::SymbolicLink => formatter.write_str("symbolic link"),
            LeftOutReason::SidecarNeedsGrouping => formatter.write_str("sidecar, needs grouping"),
            LeftOutReason::CannotBeRead(error) => write!(formatter, "cannot be read: {error}"),
        }
    }
}

/// A file or folder the scan left out, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftOut {
    /// The path relative to the source.
    pub path: PathBuf,
    /// Why it was left out.
    pub reason: LeftOutReason,
}

/// The result of a source scan: the files found and what was left out, each in the
/// byte order of their relative paths, so the same source always gives the same
/// lists.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceScan {
    /// The files to import.
    pub files: Vec<FoundFile>,
    /// The files and folders left out.
    pub left_out: Vec<LeftOut>,
}

/// Why a source cannot be scanned at all.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    /// The destination lies inside the source, so a later import would find the
    /// files this one imports (IMPORT-005.11).
    #[error(
        "the destination {} lies inside the source {}; choose a destination outside it",
        destination.display(),
        source_folder.display()
    )]
    DestinationInsideSource {
        /// The source folder.
        source_folder: PathBuf,
        /// The import destination.
        destination: PathBuf,
    },
    /// Nothing exists at the source path.
    #[error("the source {} does not exist", path.display())]
    SourceMissing {
        /// The source path.
        path: PathBuf,
    },
    /// The source path names a file or something else that is not a folder.
    #[error("the source {} is not a folder", path.display())]
    SourceNotAFolder {
        /// The source path.
        path: PathBuf,
    },
    /// The source folder itself cannot be read.
    #[error("the source {} cannot be read: {source}", path.display())]
    SourceCannotBeRead {
        /// The source path.
        path: PathBuf,
        /// The error of the file system.
        source: io::Error,
    },
}

/// Scans the folder `source` with all its subfolders (IMPORT-005).
///
/// `destination` is the import destination. A destination inside the source, or
/// equal to it, is refused before the file system is touched. Both paths are
/// compared as given, so the caller passes absolute paths without `..`.
///
/// # Errors
///
/// Returns a [`ScanError`] if the destination lies inside the source, or if the
/// source does not exist, is not a folder or cannot be read. A subfolder or file
/// that cannot be read is listed in [`SourceScan::left_out`] instead.
pub fn scan_source(
    file_system: &dyn FileSystem,
    source: &Path,
    destination: &Path,
) -> Result<SourceScan, ScanError> {
    if lies_inside(destination, source) {
        return Err(ScanError::DestinationInsideSource {
            source_folder: source.to_path_buf(),
            destination: destination.to_path_buf(),
        });
    }
    scan_folder(file_system, source)
}

/// Scans the folder `source` with all its subfolders (IMPORT-005), like
/// [`scan_source`], but without an import destination to refuse. A caller that checks
/// the destination itself uses it, such as the guided import, which shows the files of
/// a card also when the destination lies on it (CLI-003.4).
///
/// # Errors
///
/// Returns a [`ScanError`] if the source does not exist, is not a folder or cannot be
/// read.
pub fn scan_folder(file_system: &dyn FileSystem, source: &Path) -> Result<SourceScan, ScanError> {
    let entries = list_source(file_system, source)?;
    let mut scanner = Scanner {
        file_system,
        source,
        scan: SourceScan::default(),
    };
    scanner.visit_entries(Path::new(""), entries);
    let mut scan = scanner.scan;
    scan.files.sort_by(|a, b| byte_order(&a.path, &b.path));
    scan.left_out.sort_by(|a, b| byte_order(&a.path, &b.path));
    Ok(scan)
}

fn list_source(file_system: &dyn FileSystem, source: &Path) -> Result<Vec<FolderEntry>, ScanError> {
    let path = source.to_path_buf();
    match file_system.entry(source) {
        Ok(Some(EntryKind::Folder)) => {}
        Ok(Some(_)) => return Err(ScanError::SourceNotAFolder { path }),
        Ok(None) => return Err(ScanError::SourceMissing { path }),
        Err(source) => return Err(ScanError::SourceCannotBeRead { path, source }),
    }
    file_system
        .list_folder(source)
        .map_err(|source| ScanError::SourceCannotBeRead { path, source })
}

/// Whether `inner` is `outer` or lies below it, compared by path components.
fn lies_inside(inner: &Path, outer: &Path) -> bool {
    normal_components(inner).starts_with(&normal_components(outer))
}

fn normal_components(path: &Path) -> Vec<Component<'_>> {
    path.components()
        .filter(|component| *component != Component::CurDir)
        .collect()
}

fn byte_order(a: &Path, b: &Path) -> std::cmp::Ordering {
    a.as_os_str()
        .as_encoded_bytes()
        .cmp(b.as_os_str().as_encoded_bytes())
}

struct Scanner<'a> {
    file_system: &'a dyn FileSystem,
    source: &'a Path,
    scan: SourceScan,
}

impl Scanner<'_> {
    fn visit_entries(&mut self, folder: &Path, entries: Vec<FolderEntry>) {
        for entry in entries {
            let path = folder.join(&entry.name);
            self.visit(path, entry.kind);
        }
    }

    fn visit(&mut self, path: PathBuf, kind: EntryKind) {
        if is_hidden(&path) {
            return self.leave_out(path, LeftOutReason::Hidden);
        }
        match kind {
            EntryKind::Folder => self.visit_folder(path),
            EntryKind::File => self.visit_file(path),
            EntryKind::SymbolicLink => self.leave_out(path, LeftOutReason::SymbolicLink),
            EntryKind::Other => self.leave_out(path, LeftOutReason::UnsupportedType),
        }
    }

    fn visit_folder(&mut self, path: PathBuf) {
        match self.file_system.list_folder(&self.source.join(&path)) {
            Ok(entries) => self.visit_entries(&path, entries),
            Err(error) => self.leave_out(path, cannot_be_read(&error)),
        }
    }

    fn visit_file(&mut self, path: PathBuf) {
        let role = path
            .extension()
            .and_then(|extension| extension.to_str())
            .and_then(role_for_extension);
        match role {
            None => self.leave_out(path, LeftOutReason::UnsupportedType),
            Some(AssetFileRole::Sidecar) => {
                self.leave_out(path, LeftOutReason::SidecarNeedsGrouping);
            }
            Some(role) => self.find(path, role),
        }
    }

    fn find(&mut self, path: PathBuf, role: AssetFileRole) {
        let absolute = self.source.join(&path);
        let size = self.file_system.file_size(&absolute);
        let modified = self.file_system.modification_time(&absolute);
        match (size, modified) {
            (Ok(size), Ok(modification_time)) => self.scan.files.push(FoundFile {
                path,
                role,
                size,
                modification_time,
            }),
            (Err(error), _) | (_, Err(error)) => self.leave_out(path, cannot_be_read(&error)),
        }
    }

    fn leave_out(&mut self, path: PathBuf, reason: LeftOutReason) {
        self.scan.left_out.push(LeftOut { path, reason });
    }
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.as_encoded_bytes().starts_with(b"."))
}

fn cannot_be_read(error: &io::Error) -> LeftOutReason {
    LeftOutReason::CannotBeRead(error.to_string())
}
