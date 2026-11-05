// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: every adapter must pass the conformance suite of the port
//! it implements (CLAUDE.md, "Adding things").

use std::path::Path;

use adapter_free_space::StatvfsFreeSpace;
use files::FreeSpace;
use files::testing::{FreeSpaceUnderTest, TempDir, check_free_space};

#[test]
fn conforms_to_the_port() {
    let folder = TempDir::new().unwrap();

    let failures = check_free_space(&FreeSpaceUnderTest {
        free_space: &StatvfsFreeSpace::new(),
        folder: folder.path(),
    });

    assert_eq!(failures, Vec::<String>::new());
}

#[test]
fn a_path_below_a_file_is_an_error() {
    let folder = TempDir::new().unwrap();
    let file = folder.path().join("photo.jpg");
    std::fs::write(&file, b"jpeg").unwrap();
    let free_space = StatvfsFreeSpace::new();

    let below = free_space.free_space(&file.join("missing"));

    // `photo.jpg/missing` fails with "not a directory", not "not found", so it is not
    // answered for a parent.
    assert!(below.is_err());
    assert!(free_space.free_space(Path::new("/")).is_ok());
}
