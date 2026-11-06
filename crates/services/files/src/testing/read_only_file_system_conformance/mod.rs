// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`ReadOnlyFileSystem`] port (ADR 0034).
//!
//! The port cannot create what it reads, so the suite lays out its files and folders
//! through functions of the implementation's storage, such as a fake camera's.
//! [`check_file_system`](super::check_file_system) runs it with the writing methods of
//! the [`FileSystem`](crate::FileSystem) port.

use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use domain::Timestamp;

use crate::{EntryKind, ReadOnlyFileSystem};

mod listing;

/// A modification time every supported file system stores exactly: whole, even
/// seconds (exFAT stores two-second steps).
const MODIFIED_SECONDS: i64 = 1_792_245_908;

/// Creates a symbolic link at its first path that points to its second.
pub type MakeSymbolicLink<'a> = dyn Fn(&Path, &Path) -> io::Result<()> + 'a;

/// Creates the folder at its path; its parent folder exists.
pub type AddFolder<'a> = dyn Fn(&Path) -> io::Result<()> + 'a;

/// Creates the file at its path with its content and modification time; its parent
/// folder exists.
pub type AddFile<'a> = dyn Fn(&Path, &[u8], Timestamp) -> io::Result<()> + 'a;

/// What the [`ReadOnlyFileSystem`] conformance suite runs against.
pub struct ReadOnlyFileSystemUnderTest<'a> {
    /// The implementation under test.
    pub file_system: &'a dyn ReadOnlyFileSystem,
    /// An existing, empty folder the suite may add folders and files to.
    pub root: &'a Path,
    /// Adds a folder to the storage the implementation reads.
    pub add_folder: &'a AddFolder<'a>,
    /// Adds a file to the storage the implementation reads.
    pub add_file: &'a AddFile<'a>,
    /// Creates a symbolic link at its first path that points to its second, if the
    /// implementation's storage has them. The listing cases then check that links
    /// are reported and not followed.
    pub make_symbolic_link: Option<&'a MakeSymbolicLink<'a>>,
}

type CaseResult = Result<(), String>;
type CaseFn = fn(&Case<'_>) -> CaseResult;

/// Runs the conformance suite of the [`ReadOnlyFileSystem`] port and returns one
/// message per failed case, or nothing if the implementation conforms.
///
/// Each case works in its own folder below `root`.
#[must_use]
pub fn check_read_only_file_system(under_test: &ReadOnlyFileSystemUnderTest<'_>) -> Vec<String> {
    let cases: [(&str, CaseFn); 7] = [
        (
            "entry_reports_missing_files_and_folders",
            entry_reports_missing,
        ),
        (
            "list_folder_names_files_and_folders",
            listing::list_folder_names_entries,
        ),
        (
            "list_folder_does_not_follow_symbolic_links",
            listing::list_folder_does_not_follow_links,
        ),
        ("file_size_is_the_content_length", listing::file_size),
        (
            "open_read_seeks_to_any_offset",
            open_read_seeks_to_any_offset,
        ),
        ("open_read_reports_a_missing_file", open_read_missing),
        (
            "modification_time_is_reported",
            modification_time_is_reported,
        ),
    ];
    let mut failures = Vec::new();
    for (name, case) in cases {
        let folder = under_test.root.join(name);
        let result = (under_test.add_folder)(&folder)
            .map_err(|error| format!("cannot create the case folder: {error}"))
            .and_then(|()| case(&Case { under_test, folder }));
        if let Err(message) = result {
            failures.push(format!("{name}: {message}"));
        }
    }
    failures
}

struct Case<'a> {
    under_test: &'a ReadOnlyFileSystemUnderTest<'a>,
    folder: PathBuf,
}

impl Case<'_> {
    fn fs(&self) -> &dyn ReadOnlyFileSystem {
        self.under_test.file_system
    }

    fn path(&self, name: &str) -> PathBuf {
        self.folder.join(name)
    }

    fn add_folder(&self, path: &Path) -> CaseResult {
        (self.under_test.add_folder)(path).map_err(failed("add_folder"))
    }

    fn add_file(&self, path: &Path, content: &[u8]) -> CaseResult {
        (self.under_test.add_file)(path, content, modified()).map_err(failed("add_file"))
    }

    fn expect_entry(&self, path: &Path, expected: Option<EntryKind>) -> CaseResult {
        let entry = self.fs().entry(path).map_err(failed("entry"))?;
        ensure(
            entry == expected,
            format!(
                "entry of {} is {entry:?}, expected {expected:?}",
                path.display()
            ),
        )
    }
}

/// Turns an error of `operation` into the message of a failed case.
pub(super) fn failed(operation: &'static str) -> impl Fn(io::Error) -> String {
    move |error| format!("{operation} failed: {error}")
}

/// Fails the case with `message` unless `condition` holds.
pub(super) fn ensure(condition: bool, message: String) -> CaseResult {
    if condition { Ok(()) } else { Err(message) }
}

/// Fails the case unless `result` is an error of `kind`.
pub(super) fn expect_error_kind<T>(
    result: io::Result<T>,
    kind: io::ErrorKind,
    operation: &str,
) -> CaseResult {
    match result {
        Ok(_) => Err(format!("{operation} succeeded, expected {kind:?}")),
        Err(error) if error.kind() == kind => Ok(()),
        Err(error) => Err(format!(
            "{operation} failed with {:?} ({error}), expected {kind:?}",
            error.kind()
        )),
    }
}

/// The modification time the suites give their files.
pub(super) fn modified() -> Timestamp {
    Timestamp::from_unix_seconds(MODIFIED_SECONDS)
}

fn entry_reports_missing(case: &Case<'_>) -> CaseResult {
    case.expect_entry(&case.path("missing"), None)?;
    case.expect_entry(&case.folder, Some(EntryKind::Folder))
}

/// Metadata readers read byte ranges at offsets and never a whole RAW file (ADR 0021).
fn open_read_seeks_to_any_offset(case: &Case<'_>) -> CaseResult {
    let path = case.path("photo.raf");
    case.add_file(&path, b"0123456789")?;
    let mut reader = case.fs().open_read(&path).map_err(failed("open_read"))?;
    let mut read_at = |position: SeekFrom| -> Result<(u64, Vec<u8>), String> {
        let offset = reader.seek(position).map_err(failed("seek"))?;
        let mut bytes = [0; 2];
        let count = reader.read(&mut bytes).map_err(failed("read"))?;
        Ok((offset, bytes[..count].to_vec()))
    };

    let reads = [
        read_at(SeekFrom::Start(6))?,
        read_at(SeekFrom::Current(-5))?,
        read_at(SeekFrom::End(-1))?,
        read_at(SeekFrom::Start(20))?,
    ];
    let expected = [
        (6, b"67".to_vec()),
        (3, b"34".to_vec()),
        (9, b"9".to_vec()),
        (20, Vec::new()),
    ];
    ensure(
        reads == expected,
        format!("seek and read gave {reads:?}, expected {expected:?}"),
    )?;
    ensure(
        reader.seek(SeekFrom::End(-11)).is_err(),
        "seeking before the start succeeded".to_owned(),
    )
}

fn open_read_missing(case: &Case<'_>) -> CaseResult {
    expect_error_kind(
        case.fs().open_read(&case.path("missing")),
        io::ErrorKind::NotFound,
        "open_read",
    )?;
    expect_error_kind(
        case.fs().modification_time(&case.path("missing")),
        io::ErrorKind::NotFound,
        "modification_time",
    )
}

fn modification_time_is_reported(case: &Case<'_>) -> CaseResult {
    let path = case.path("photo.raf");
    case.add_file(&path, b"raw")?;
    let time = case
        .fs()
        .modification_time(&path)
        .map_err(failed("modification_time"))?;
    ensure(
        time == modified(),
        format!("modification time is {time:?}, expected {:?}", modified()),
    )
}
