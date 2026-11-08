// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A file in the way of a target folder (IMPORT-017), against `MemoryFileSystem`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::Path;

use files::testing::MemoryFileSystem;
use import::{ConflictOutcome, ConflictPolicy};
use support::{DESTINATION, POLICIES, add, copy_to, planned, resolve_all};

mod support;

fn folder_is_a_file(target: &str, folder: &str) -> ConflictOutcome {
    ConflictOutcome::TargetFolderIsAFile {
        target: target.to_owned(),
        folder: Path::new(DESTINATION).join(folder),
    }
}

/// spec: IMPORT-017.1
#[test]
fn a_file_where_a_target_folder_should_be() {
    for policy in POLICIES {
        let file_system = MemoryFileSystem::new();
        let files = [planned(
            &file_system,
            "DSCF0001.RAF",
            b"ours",
            "2026/DSCF0001.RAF",
        )];
        add(&file_system, DESTINATION, "2026", b"a file");

        let outcomes = resolve_all(&file_system, policy, &files);

        assert_eq!(
            outcomes,
            [folder_is_a_file("2026/DSCF0001.RAF", "2026")],
            "{policy:?}"
        );
        assert_eq!(
            outcomes[0].to_string(),
            "failed: the target folder /library/2026 is a file"
        );
    }
}

/// spec: IMPORT-017.2
#[test]
fn a_file_deeper_in_the_target_path() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(
        &file_system,
        "DSCF0001.RAF",
        b"ours",
        "2026/10/DSCF0001.RAF",
    )];
    add(&file_system, DESTINATION, "2026/10", b"a file");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(
        outcomes,
        [folder_is_a_file("2026/10/DSCF0001.RAF", "2026/10")]
    );
}

/// spec: IMPORT-017.3
#[test]
fn the_file_in_the_way_is_named_as_found() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(
        &file_system,
        "DSCF0001.JPG",
        b"ours",
        "Café/DSCF0001.JPG",
    )];
    add(&file_system, DESTINATION, "CAFÉ", b"a file");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [folder_is_a_file("Café/DSCF0001.JPG", "CAFÉ")]);
}

/// A missing target folder is created by the copy, so it is no file in the way.
#[test]
fn a_missing_target_folder_is_free() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(
        &file_system,
        "DSCF0001.RAF",
        b"ours",
        "2026/DSCF0001.RAF",
    )];
    add(&file_system, DESTINATION, "2025", b"another file");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [copy_to("2026/DSCF0001.RAF")]);
}
