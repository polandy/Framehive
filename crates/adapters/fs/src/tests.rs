// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The fallback of `rename_no_replace` for file systems without hard links, with the
//! link replaced by one that fails as such a file system does.

use std::fs;
use std::io;
use std::path::Path;

use files::FileSystem;
use files::testing::{FileSystemUnderTest, TempDir, check_file_system};

use super::*;

fn link_denied(_: &Path, _: &Path) -> io::Result<()> {
    // exFAT answers a hard link with EPERM.
    Err(io::Error::from(io::ErrorKind::PermissionDenied))
}

fn link_unsupported(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

fn link_crosses_devices(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::from(io::ErrorKind::CrossesDevices))
}

fn without_links(link: LinkFn) -> StdFileSystem {
    StdFileSystem { link }
}

#[test]
fn passes_the_conformance_suite_without_hard_links() {
    for link in [link_denied as LinkFn, link_unsupported] {
        let folder = TempDir::new().unwrap();
        let failures = check_file_system(&FileSystemUnderTest {
            file_system: &without_links(link),
            root: folder.path(),
            crash: None,
            make_symbolic_link: None,
        });
        assert!(failures.is_empty(), "{failures:#?}");
    }
}

#[test]
fn renames_without_hard_links() {
    let folder = TempDir::new().unwrap();
    let from = folder.path().join("photo.raf.part");
    let to = folder.path().join("photo.raf");
    fs::write(&from, b"raw").unwrap();
    without_links(link_denied)
        .rename_no_replace(&from, &to)
        .unwrap();
    assert!(!from.exists());
    assert_eq!(fs::read(&to).unwrap(), b"raw");
}

#[test]
fn never_replaces_an_existing_file_without_hard_links() {
    let folder = TempDir::new().unwrap();
    let from = folder.path().join("photo.raf.part");
    let to = folder.path().join("photo.raf");
    fs::write(&from, b"ours").unwrap();
    fs::write(&to, b"theirs").unwrap();
    let error = without_links(link_denied)
        .rename_no_replace(&from, &to)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read(&from).unwrap(), b"ours");
    assert_eq!(fs::read(&to).unwrap(), b"theirs");
}

#[test]
fn reports_a_missing_source_without_hard_links() {
    let folder = TempDir::new().unwrap();
    let error = without_links(link_denied)
        .rename_no_replace(&folder.path().join("missing"), &folder.path().join("to"))
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

#[test]
fn does_not_rename_a_folder_without_hard_links() {
    let folder = TempDir::new().unwrap();
    let from = folder.path().join("folder");
    fs::create_dir(&from).unwrap();
    let to = folder.path().join("moved");
    let error = without_links(link_denied)
        .rename_no_replace(&from, &to)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert!(from.is_dir());
    assert!(!to.exists());
}

#[test]
fn other_link_errors_are_reported_without_renaming() {
    let folder = TempDir::new().unwrap();
    let from = folder.path().join("photo.raf.part");
    let to = folder.path().join("photo.raf");
    fs::write(&from, b"raw").unwrap();
    let error = without_links(link_crosses_devices)
        .rename_no_replace(&from, &to)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::CrossesDevices);
    assert_eq!(fs::read(&from).unwrap(), b"raw");
    assert!(!to.exists());
}
