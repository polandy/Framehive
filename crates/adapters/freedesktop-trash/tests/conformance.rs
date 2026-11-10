// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: every adapter must pass the conformance suite of the port
//! it implements (CLAUDE.md, "Adding things").

use std::fs;

use adapter_freedesktop_trash::FreedesktopTrash;
use files::testing::{TempDir, TrashUnderTest, check_trash};

mod support;

use support::{clock, write_file};

#[test]
fn conforms_to_the_port() {
    let folder = TempDir::new().unwrap();
    let root = folder.path().join("photos");
    fs::create_dir(&root).unwrap();
    let trash = FreedesktopTrash::new(folder.path().join("Trash"), clock());

    let failures = check_trash(&TrashUnderTest {
        trash: &trash,
        root: &root,
        write_file: &write_file,
        read_file: &|path| fs::read(path),
        remove_file: &|path| fs::remove_file(path),
    });

    assert_eq!(failures, Vec::<String>::new());
}
