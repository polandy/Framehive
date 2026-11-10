// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: every adapter must pass the conformance suite of the port
//! it implements (CLAUDE.md, "Adding things").
//!
//! macOS cannot point `trashItemAtURL` at another folder, so this suite uses the
//! user's Trash and removes every file it put there again (ADR 0043 §4).

#![cfg(target_os = "macos")]

use std::fs;
use std::path::Path;

use adapter_macos_trash::MacosTrash;
use files::Trash;
use files::testing::{TempDir, TrashUnderTest, check_trash};

fn write_file(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

#[test]
fn conforms_to_the_port() {
    let folder = TempDir::new().unwrap();

    let failures = check_trash(&TrashUnderTest {
        trash: &MacosTrash::new(),
        root: folder.path(),
        write_file: &write_file,
        read_file: &|path| fs::read(path),
        remove_file: &|path| fs::remove_file(path),
    });

    assert_eq!(failures, Vec::<String>::new());
}

#[test]
fn a_symbolic_link_is_refused_and_stays() {
    let folder = TempDir::new().unwrap();
    let link = folder.path().join("link.RAF");
    std::os::unix::fs::symlink("DSCF0001.RAF", &link).unwrap();

    let error = MacosTrash::new().move_to_trash(&link).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(fs::symlink_metadata(&link).is_ok());
}
