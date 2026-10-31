// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: [`StdFileSystem`] passes the conformance suite of the
//! `files::FileSystem` port on the real file system, in a temporary folder (CLAUDE.md,
//! "Adding things").

use adapter_fs::StdFileSystem;
use files::testing::{FileSystemUnderTest, TempDir, check_file_system};

#[test]
fn conforms_to_the_file_system_port() {
    let folder = TempDir::new().unwrap();
    let failures = check_file_system(&FileSystemUnderTest {
        file_system: &StdFileSystem::new(),
        root: folder.path(),
        // A crash cannot be simulated on a real file system.
        crash: None,
    });
    assert!(failures.is_empty(), "{failures:#?}");
}

#[cfg(unix)]
#[test]
fn reports_symbolic_links_as_other_entries() {
    use files::{EntryKind, FileSystem};
    let folder = TempDir::new().unwrap();
    let target = folder.path().join("photo.jpg");
    std::fs::write(&target, b"jpeg").unwrap();
    let link = folder.path().join("link.jpg");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert_eq!(
        StdFileSystem::new().entry(&link).unwrap(),
        Some(EntryKind::Other)
    );
}
