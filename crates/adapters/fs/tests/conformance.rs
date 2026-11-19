// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: [`StdFileSystem`] passes the conformance suite of the
//! `files::FileSystem` port on the real file system, in a temporary folder (CLAUDE.md,
//! "Adding things").

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use adapter_fs::StdFileSystem;
use files::testing::{FileSystemUnderTest, MakeSymbolicLink, TempDir, check_file_system};

/// spec: IMPORT-004.14
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

    use files::ReadOnlyFileSystem;

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

/// A replaced file keeps its permissions, and a read-only file is refused for the
/// superuser too (CORE-010.5).
#[cfg(unix)]
#[test]
fn replace_file_keeps_permissions_and_refuses_a_read_only_file() {
    use std::fs::{self, Permissions};
    use std::os::unix::fs::PermissionsExt;

    use files::FileSystem;

    let folder = TempDir::new().unwrap();
    let part = folder.path().join("photo.raf.xmp.part");
    let target = folder.path().join("photo.raf.xmp");
    let mode = |path: &std::path::Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    fs::write(&part, b"ours").unwrap();
    fs::write(&target, b"theirs").unwrap();
    fs::set_permissions(&part, Permissions::from_mode(0o644)).unwrap();
    fs::set_permissions(&target, Permissions::from_mode(0o600)).unwrap();

    StdFileSystem::new().replace_file(&part, &target).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"ours");
    assert_eq!(mode(&target), 0o600);

    fs::write(&part, b"newer").unwrap();
    fs::set_permissions(&target, Permissions::from_mode(0o444)).unwrap();
    let refused = StdFileSystem::new().replace_file(&part, &target);
    assert_eq!(
        refused.map_err(|error| error.kind()),
        Err(std::io::ErrorKind::PermissionDenied)
    );
    assert_eq!(fs::read(&target).unwrap(), b"ours");
    assert_eq!(mode(&target), 0o444);
    fs::set_permissions(&target, Permissions::from_mode(0o644)).unwrap();
}

/// A copy into a folder without write permission fails while it writes the `.part`
/// file and leaves neither it nor the target. The superuser may write there; then the
/// copy succeeds as `std::fs::write` does.
/// spec: IMPORT-004.5
#[cfg(unix)]
#[test]
fn a_copy_into_a_read_only_folder_leaves_nothing_behind() {
    use std::fs;

    use files::testing::FnvHasher;
    use files::{CopyErrorKind, CopyStep, copy_verified};

    let folder = TempDir::new().unwrap();
    let source = folder.path().join("DSCF0001.RAF");
    fs::write(&source, b"raw").unwrap();
    let locked = read_only_folder(folder.path());
    let target = locked.join("DSCF0001.RAF");

    let copied = copy_verified(&StdFileSystem::new(), &FnvHasher, &source, &target);
    let superuser = can_write_in(&locked);
    let target_content = fs::read(&target).ok();
    let part_left = locked.join("DSCF0001.RAF.part").exists();
    unlock(&locked);

    if superuser {
        assert!(copied.is_ok(), "{copied:?}");
        assert_eq!(target_content.as_deref(), Some(&b"raw"[..]));
    } else {
        let error = copied.unwrap_err();
        assert_eq!(error.step, CopyStep::WritePart);
        assert!(
            matches!(&error.kind, CopyErrorKind::Io { error, .. } if error.kind() == std::io::ErrorKind::PermissionDenied),
            "{error}"
        );
        assert_eq!(target_content, None);
    }
    assert!(!part_left);
}

/// A rename into a folder without write permission is refused and keeps the `.part`
/// file; it does not take the path for file systems without hard links (ADR 0020 §3).
#[cfg(unix)]
#[test]
fn a_rename_into_a_read_only_folder_keeps_the_part_file() {
    use std::fs;

    use files::FileSystem;

    let folder = TempDir::new().unwrap();
    let part = folder.path().join("DSCF0001.RAF.part");
    fs::write(&part, b"raw").unwrap();
    let locked = read_only_folder(folder.path());
    let target = locked.join("DSCF0001.RAF");

    let renamed = StdFileSystem::new().rename_no_replace(&part, &target);
    let superuser = can_write_in(&locked);
    unlock(&locked);

    if superuser {
        assert!(renamed.is_ok(), "{renamed:?}");
        assert_eq!(fs::read(&target).unwrap(), b"raw");
    } else {
        assert_eq!(
            renamed.map_err(|error| error.raw_os_error()),
            Err(Some(libc::EACCES))
        );
        assert_eq!(fs::read(&part).unwrap(), b"raw");
        assert!(!target.exists());
    }
}

/// Creates `locked` in `parent` with read and search, but no write permission.
#[cfg(unix)]
fn read_only_folder(parent: &std::path::Path) -> std::path::PathBuf {
    use std::fs::{self, Permissions};
    use std::os::unix::fs::PermissionsExt;

    let locked = parent.join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, Permissions::from_mode(0o555)).unwrap();
    locked
}

/// Whether `std::fs` may create a file in `folder`, which the superuser may always.
#[cfg(unix)]
fn can_write_in(folder: &std::path::Path) -> bool {
    let probe = folder.join("probe");
    let written = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(probe);
    written
}

/// Gives the owner write permission again, so that the temporary folder can be removed.
#[cfg(unix)]
fn unlock(folder: &std::path::Path) {
    use std::fs::{self, Permissions};
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(folder, Permissions::from_mode(0o755)).unwrap();
}
