// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`FileSystem`] port.

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use domain::Timestamp;

use crate::{EntryKind, FileSystem};

/// A modification time every supported file system stores exactly: whole, even
/// seconds (exFAT stores two-second steps).
const MODIFIED_SECONDS: i64 = 1_792_245_908;

/// What the [`FileSystem`] conformance suite runs against.
pub struct FileSystemUnderTest<'a> {
    /// The implementation under test.
    pub file_system: &'a dyn FileSystem,
    /// An existing, empty folder the suite may write to.
    pub root: &'a Path,
    /// Simulates a crash that loses every write that was not synced, if the
    /// implementation can; the durability cases then check what survives it.
    pub crash: Option<&'a dyn Fn()>,
}

type CaseResult = Result<(), String>;
type CaseFn = fn(&Case<'_>) -> CaseResult;

/// Runs the conformance suite of the [`FileSystem`] port and returns one message per
/// failed case, or nothing if the implementation conforms.
///
/// Each case works in its own folder below `root`.
#[must_use]
pub fn check_file_system(under_test: &FileSystemUnderTest<'_>) -> Vec<String> {
    let cases: [(&str, CaseFn); 13] = [
        (
            "entry_reports_missing_files_and_folders",
            entry_reports_missing,
        ),
        ("create_folder_needs_a_parent_and_no_entry", create_folder),
        ("written_content_reads_back", written_content_reads_back),
        (
            "open_read_seeks_to_any_offset",
            open_read_seeks_to_any_offset,
        ),
        ("create_file_empties_an_existing_file", create_file_empties),
        (
            "create_file_needs_a_parent_folder",
            create_file_needs_parent,
        ),
        ("open_read_reports_a_missing_file", open_read_missing),
        ("modification_time_is_kept", modification_time_is_kept),
        ("rename_moves_the_file", rename_moves),
        (
            "rename_never_replaces_an_existing_file",
            rename_never_replaces,
        ),
        ("remove_file_removes_only_that_file", remove_file),
        (
            "sync_keeps_content_and_reports_a_missing_folder",
            sync_keeps_content,
        ),
        (
            "synced_rename_survives_a_crash",
            synced_rename_survives_a_crash,
        ),
    ];
    let mut failures = Vec::new();
    for (name, case) in cases {
        let folder = under_test.root.join(name);
        let result = under_test
            .file_system
            .create_folder(&folder)
            .map_err(|error| format!("cannot create the case folder: {error}"))
            .and_then(|()| case(&Case { under_test, folder }));
        if let Err(message) = result {
            failures.push(format!("{name}: {message}"));
        }
    }
    failures
}

struct Case<'a> {
    under_test: &'a FileSystemUnderTest<'a>,
    folder: PathBuf,
}

impl Case<'_> {
    fn fs(&self) -> &dyn FileSystem {
        self.under_test.file_system
    }

    fn path(&self, name: &str) -> PathBuf {
        self.folder.join(name)
    }

    fn write(&self, path: &Path, content: &[u8]) -> CaseResult {
        let mut file = self.fs().create_file(path).map_err(failed("create_file"))?;
        file.write_all(content).map_err(failed("write"))?;
        file.flush().map_err(failed("flush"))
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, String> {
        let mut content = Vec::new();
        self.fs()
            .open_read(path)
            .map_err(failed("open_read"))?
            .read_to_end(&mut content)
            .map_err(failed("read"))?;
        Ok(content)
    }

    fn expect_content(&self, path: &Path, expected: &[u8]) -> CaseResult {
        let content = self.read(path)?;
        ensure(
            content == expected,
            format!(
                "{} has {} bytes, expected {}",
                path.display(),
                content.len(),
                expected.len()
            ),
        )
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

fn failed(operation: &'static str) -> impl Fn(io::Error) -> String {
    move |error| format!("{operation} failed: {error}")
}

fn ensure(condition: bool, message: String) -> CaseResult {
    if condition { Ok(()) } else { Err(message) }
}

fn expect_error_kind<T>(result: io::Result<T>, kind: io::ErrorKind, operation: &str) -> CaseResult {
    match result {
        Ok(_) => Err(format!("{operation} succeeded, expected {kind:?}")),
        Err(error) if error.kind() == kind => Ok(()),
        Err(error) => Err(format!(
            "{operation} failed with {:?} ({error}), expected {kind:?}",
            error.kind()
        )),
    }
}

fn modified() -> Timestamp {
    Timestamp::from_unix_seconds(MODIFIED_SECONDS)
}

fn entry_reports_missing(case: &Case<'_>) -> CaseResult {
    case.expect_entry(&case.path("missing"), None)?;
    case.expect_entry(&case.folder, Some(EntryKind::Folder))
}

fn create_folder(case: &Case<'_>) -> CaseResult {
    let folder = case.path("2026");
    case.fs()
        .create_folder(&folder)
        .map_err(failed("create_folder"))?;
    case.expect_entry(&folder, Some(EntryKind::Folder))?;
    expect_error_kind(
        case.fs().create_folder(&folder),
        io::ErrorKind::AlreadyExists,
        "creating it again",
    )?;
    expect_error_kind(
        case.fs().create_folder(&case.path("missing/2026")),
        io::ErrorKind::NotFound,
        "creating a folder in a missing folder",
    )
}

fn written_content_reads_back(case: &Case<'_>) -> CaseResult {
    let path = case.path("photo.raf");
    let content: Vec<u8> = (0..=255).cycle().take(100_000).collect();
    case.write(&path, &content)?;
    case.expect_entry(&path, Some(EntryKind::File))?;
    case.expect_content(&path, &content)
}

/// Metadata readers read byte ranges at offsets and never a whole RAW file (ADR 0021).
fn open_read_seeks_to_any_offset(case: &Case<'_>) -> CaseResult {
    let path = case.path("photo.raf");
    case.write(&path, b"0123456789")?;
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

fn create_file_empties(case: &Case<'_>) -> CaseResult {
    let path = case.path("photo.raf.part");
    case.write(&path, b"left by an interrupted import")?;
    case.write(&path, b"new")?;
    case.expect_content(&path, b"new")
}

fn create_file_needs_parent(case: &Case<'_>) -> CaseResult {
    expect_error_kind(
        case.fs().create_file(&case.path("missing/photo.raf")),
        io::ErrorKind::NotFound,
        "creating a file in a missing folder",
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

fn modification_time_is_kept(case: &Case<'_>) -> CaseResult {
    let path = case.path("photo.raf");
    let mut file = case
        .fs()
        .create_file(&path)
        .map_err(failed("create_file"))?;
    file.write_all(b"raw").map_err(failed("write"))?;
    file.set_modification_time(modified())
        .map_err(failed("set_modification_time"))?;
    file.sync().map_err(failed("sync"))?;
    drop(file);
    let time = case
        .fs()
        .modification_time(&path)
        .map_err(failed("modification_time"))?;
    ensure(
        time == modified(),
        format!("modification time is {time:?}, expected {:?}", modified()),
    )
}

fn rename_moves(case: &Case<'_>) -> CaseResult {
    let from = case.path("photo.raf.part");
    let to = case.path("photo.raf");
    case.write(&from, b"raw")?;
    case.fs()
        .rename_no_replace(&from, &to)
        .map_err(failed("rename_no_replace"))?;
    case.expect_entry(&from, None)?;
    case.expect_content(&to, b"raw")?;
    expect_error_kind(
        case.fs().rename_no_replace(&from, &case.path("other.raf")),
        io::ErrorKind::NotFound,
        "renaming a missing file",
    )
}

fn rename_never_replaces(case: &Case<'_>) -> CaseResult {
    let from = case.path("photo.raf.part");
    let to = case.path("photo.raf");
    case.write(&from, b"ours")?;
    case.write(&to, b"theirs")?;
    expect_error_kind(
        case.fs().rename_no_replace(&from, &to),
        io::ErrorKind::AlreadyExists,
        "rename_no_replace",
    )?;
    case.expect_content(&from, b"ours")?;
    case.expect_content(&to, b"theirs")
}

fn remove_file(case: &Case<'_>) -> CaseResult {
    let removed = case.path("photo.raf.part");
    let kept = case.path("photo.raf");
    case.write(&removed, b"part")?;
    case.write(&kept, b"raw")?;
    case.fs()
        .remove_file(&removed)
        .map_err(failed("remove_file"))?;
    case.expect_entry(&removed, None)?;
    case.expect_content(&kept, b"raw")?;
    expect_error_kind(
        case.fs().remove_file(&removed),
        io::ErrorKind::NotFound,
        "removing it again",
    )
}

fn sync_keeps_content(case: &Case<'_>) -> CaseResult {
    let path = case.path("photo.raf");
    let mut file = case
        .fs()
        .create_file(&path)
        .map_err(failed("create_file"))?;
    file.write_all(b"raw").map_err(failed("write"))?;
    file.flush().map_err(failed("flush"))?;
    file.sync().map_err(failed("sync"))?;
    drop(file);
    case.fs()
        .sync_folder(&case.folder)
        .map_err(failed("sync_folder"))?;
    case.expect_content(&path, b"raw")?;
    ensure(
        case.fs().sync_folder(&case.path("missing")).is_err(),
        "sync_folder of a missing folder succeeded".to_owned(),
    )
}

/// The steps of a verified copy, then a crash: the synced, renamed file is complete.
fn synced_rename_survives_a_crash(case: &Case<'_>) -> CaseResult {
    let root = case.under_test.root;
    case.fs()
        .sync_folder(root)
        .map_err(failed("sync_folder of the root"))?;
    let part = case.path("photo.raf.part");
    let target = case.path("photo.raf");
    let content: Vec<u8> = (0..=255).rev().cycle().take(70_000).collect();
    let mut file = case
        .fs()
        .create_file(&part)
        .map_err(failed("create_file"))?;
    file.write_all(&content).map_err(failed("write"))?;
    file.flush().map_err(failed("flush"))?;
    file.set_modification_time(modified())
        .map_err(failed("set_modification_time"))?;
    file.sync().map_err(failed("sync"))?;
    drop(file);
    case.fs()
        .rename_no_replace(&part, &target)
        .map_err(failed("rename_no_replace"))?;
    case.fs()
        .sync_folder(&case.folder)
        .map_err(failed("sync_folder"))?;
    if let Some(crash) = case.under_test.crash {
        crash();
    }
    case.expect_entry(&part, None)?;
    case.expect_content(&target, &content)?;
    let time = case
        .fs()
        .modification_time(&target)
        .map_err(failed("modification_time"))?;
    ensure(
        time == modified(),
        format!("modification time after the crash is {time:?}"),
    )
}
