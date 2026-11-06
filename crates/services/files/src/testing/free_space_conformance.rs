// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`FreeSpace`] port.

use std::path::Path;

use crate::{FileSystemId, FreeSpace};

/// What the [`FreeSpace`] conformance suite runs against.
pub struct FreeSpaceUnderTest<'a> {
    /// The implementation under test.
    pub free_space: &'a dyn FreeSpace,
    /// An existing folder, writable by the test, on a file system that reports its
    /// free space. The suite only asks about it and paths below it; it writes nothing.
    pub folder: &'a Path,
}

/// Runs the conformance suite of the [`FreeSpace`] port and returns one message per
/// failed check, or nothing if the implementation conforms.
///
/// The free space itself may change between two answers, because other programs
/// write to the same file system, so the suite compares only file system identities.
#[must_use]
pub fn check_free_space(under_test: &FreeSpaceUnderTest<'_>) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(folder) = file_system_of(under_test, under_test.folder, &mut failures) else {
        return failures;
    };
    check_stable_identity(under_test, folder, &mut failures);
    check_missing_folders(under_test, folder, &mut failures);
    failures
}

fn file_system_of(
    under_test: &FreeSpaceUnderTest<'_>,
    path: &Path,
    failures: &mut Vec<String>,
) -> Option<FileSystemId> {
    match under_test.free_space.free_space(path) {
        Ok(space) => Some(space.file_system),
        Err(error) => {
            failures.push(format!(
                "free_space: {} cannot be answered: {error}",
                path.display()
            ));
            None
        }
    }
}

/// Asking twice about one folder names the same file system.
fn check_stable_identity(
    under_test: &FreeSpaceUnderTest<'_>,
    folder: FileSystemId,
    failures: &mut Vec<String>,
) {
    let again = file_system_of(under_test, under_test.folder, failures);
    if again.is_some_and(|again| again != folder) {
        failures.push("stable identity: two answers for one folder differ".to_owned());
    }
}

/// A folder that does not exist yet, one or several levels down, is answered for its
/// nearest existing parent folder.
fn check_missing_folders(
    under_test: &FreeSpaceUnderTest<'_>,
    folder: FileSystemId,
    failures: &mut Vec<String>,
) {
    for missing in ["missing", "missing/below/deeper"] {
        let path = under_test.folder.join(missing);
        let answer = file_system_of(under_test, &path, failures);
        if answer.is_some_and(|answer| answer != folder) {
            failures.push(format!(
                "missing folder: {} is not on the file system of its existing parent",
                path.display()
            ));
        }
    }
}
