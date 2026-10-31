// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: [`StdFileSystem`] passes the conformance suite of the
//! `files::FileSystem` port on the real file system, in a temporary folder (CLAUDE.md,
//! "Adding things").

use adapter_fs::StdFileSystem;
use files::testing::{FileSystemUnderTest, MakeSymbolicLink, TempDir, check_file_system};

#[test]
fn conforms_to_the_file_system_port() {
    let folder = TempDir::new().unwrap();
    let failures = check_file_system(&FileSystemUnderTest {
        file_system: &StdFileSystem::new(),
        root: folder.path(),
        // A crash cannot be simulated on a real file system.
        crash: None,
        make_symbolic_link: SYMBOLIC_LINKS,
    });
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Symbolic links of the platform, for the listing cases of the suite.
#[cfg(unix)]
const SYMBOLIC_LINKS: Option<&MakeSymbolicLink<'static>> =
    Some(&|link, target| std::os::unix::fs::symlink(target, link));

#[cfg(not(unix))]
const SYMBOLIC_LINKS: Option<&MakeSymbolicLink<'static>> = None;

/// A folder without read permission fails to list as `std::fs::read_dir` does, so
/// the scan can list it as left out (IMPORT-005.6). Permissions do not bind the
/// superuser; then both listings succeed, which the comparison covers too.
#[cfg(unix)]
#[test]
fn list_folder_fails_like_read_dir_for_a_folder_without_permission() {
    use std::fs::{self, Permissions};
    use std::os::unix::fs::PermissionsExt;

    use files::FileSystem;

    let folder = TempDir::new().unwrap();
    let locked = folder.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::write(locked.join("DSCF0001.RAF"), b"raw").unwrap();
    fs::set_permissions(&locked, Permissions::from_mode(0o000)).unwrap();

    let expected = fs::read_dir(&locked)
        .map(|_| ())
        .map_err(|error| error.kind());
    let listed = StdFileSystem::new()
        .list_folder(&locked)
        .map(|_| ())
        .map_err(|error| error.kind());
    fs::set_permissions(&locked, Permissions::from_mode(0o755)).unwrap();

    assert_eq!(listed, expected);
    if let Err(kind) = listed {
        assert_eq!(kind, std::io::ErrorKind::PermissionDenied);
    }
}
