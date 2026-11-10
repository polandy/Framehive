// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`Trash`] port (ADR 0043).
//!
//! The port only moves files, so the suite lays out and inspects its files through
//! functions of the implementation's storage, and removes every file it moved to the
//! trash again, so a real trash is left as it was found.

use std::io;
use std::path::{Path, PathBuf};

use crate::Trash;

/// Creates the file at its path with its content, and its missing parent folders.
pub type WriteTrashFile<'a> = dyn Fn(&Path, &[u8]) -> io::Result<()> + 'a;

/// Reads the file at its path in full; [`io::ErrorKind::NotFound`] if it is missing.
pub type ReadTrashFile<'a> = dyn Fn(&Path) -> io::Result<Vec<u8>> + 'a;

/// Removes the file at its path.
pub type RemoveTrashFile<'a> = dyn Fn(&Path) -> io::Result<()> + 'a;

/// What the [`Trash`] conformance suite runs against.
pub struct TrashUnderTest<'a> {
    /// The implementation under test.
    pub trash: &'a dyn Trash,
    /// An existing, empty folder the suite may add files to.
    pub root: &'a Path,
    /// Adds a file to the storage the trash moves files from.
    pub write_file: &'a WriteTrashFile<'a>,
    /// Reads a file of that storage, below `root` or in the trash.
    pub read_file: &'a ReadTrashFile<'a>,
    /// Removes a file the suite moved to the trash.
    pub remove_file: &'a RemoveTrashFile<'a>,
}

type CaseResult = Result<(), String>;
type CaseFn = fn(&TrashUnderTest<'_>, &Path, &mut Vec<PathBuf>) -> CaseResult;

/// Runs the conformance suite of the [`Trash`] port and returns one message per
/// failed case, or nothing if the implementation conforms.
///
/// Each case works in its own folder below `root`.
#[must_use]
pub fn check_trash(under_test: &TrashUnderTest<'_>) -> Vec<String> {
    let cases: [(&str, CaseFn); 4] = [
        ("moves_a_file_with_its_content", moves_a_file),
        ("a_file_of_the_same_name_gets_its_own_place", same_name),
        ("a_missing_file_is_not_found", missing_file),
        ("a_folder_is_refused", folder_refused),
    ];
    let mut failures = Vec::new();
    for (name, case) in cases {
        let mut trashed = Vec::new();
        let result = case(under_test, &under_test.root.join(name), &mut trashed);
        if let Err(message) = result {
            failures.push(format!("{name}: {message}"));
        }
        for path in trashed {
            if let Err(error) = (under_test.remove_file)(&path) {
                failures.push(format!("{name}: cannot remove {}: {error}", path.display()));
            }
        }
    }
    failures
}

fn moves_a_file(
    under_test: &TrashUnderTest<'_>,
    folder: &Path,
    trashed: &mut Vec<PathBuf>,
) -> CaseResult {
    let file = folder.join("DSCF0001.RAF");
    write(under_test, &file, b"raw")?;
    let in_trash = move_to_trash(under_test, &file, trashed)?;
    expect_missing(under_test, &file)?;
    expect_content(under_test, &in_trash, b"raw")
}

fn same_name(
    under_test: &TrashUnderTest<'_>,
    folder: &Path,
    trashed: &mut Vec<PathBuf>,
) -> CaseResult {
    let first = folder.join("one/DSCF0001.RAF");
    let second = folder.join("two/DSCF0001.RAF");
    write(under_test, &first, b"first")?;
    write(under_test, &second, b"second")?;
    let first_in_trash = move_to_trash(under_test, &first, trashed)?;
    let second_in_trash = move_to_trash(under_test, &second, trashed)?;
    if first_in_trash == second_in_trash {
        return Err(format!("both lie at {}", first_in_trash.display()));
    }
    expect_content(under_test, &first_in_trash, b"first")?;
    expect_content(under_test, &second_in_trash, b"second")
}

fn missing_file(
    under_test: &TrashUnderTest<'_>,
    folder: &Path,
    trashed: &mut Vec<PathBuf>,
) -> CaseResult {
    match under_test.trash.move_to_trash(&folder.join("DSCF0001.RAF")) {
        Ok(in_trash) => {
            trashed.push(in_trash);
            Err("a missing file was moved".to_owned())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("expected NotFound, got {error}")),
    }
}

fn folder_refused(
    under_test: &TrashUnderTest<'_>,
    folder: &Path,
    trashed: &mut Vec<PathBuf>,
) -> CaseResult {
    let inner = folder.join("folder/DSCF0001.RAF");
    write(under_test, &inner, b"raw")?;
    match under_test.trash.move_to_trash(&folder.join("folder")) {
        Ok(in_trash) => {
            trashed.push(in_trash);
            Err("a folder was moved".to_owned())
        }
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => {
            expect_content(under_test, &inner, b"raw")
        }
        Err(error) => Err(format!("expected InvalidInput, got {error}")),
    }
}

fn write(under_test: &TrashUnderTest<'_>, path: &Path, content: &[u8]) -> CaseResult {
    (under_test.write_file)(path, content)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn move_to_trash(
    under_test: &TrashUnderTest<'_>,
    path: &Path,
    trashed: &mut Vec<PathBuf>,
) -> Result<PathBuf, String> {
    let in_trash = under_test
        .trash
        .move_to_trash(path)
        .map_err(|error| format!("cannot move {} to the trash: {error}", path.display()))?;
    trashed.push(in_trash.clone());
    Ok(in_trash)
}

fn expect_missing(under_test: &TrashUnderTest<'_>, path: &Path) -> CaseResult {
    match (under_test.read_file)(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(format!("{} is still there", path.display())),
        Err(error) => Err(format!("cannot inspect {}: {error}", path.display())),
    }
}

fn expect_content(under_test: &TrashUnderTest<'_>, path: &Path, expected: &[u8]) -> CaseResult {
    match (under_test.read_file)(path) {
        Ok(content) if content == expected => Ok(()),
        Ok(_) => Err(format!("{} holds other bytes", path.display())),
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}
