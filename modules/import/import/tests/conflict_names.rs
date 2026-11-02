// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Target names that differ only in case or in Unicode normalization form
//! (IMPORT-003), against `MemoryFileSystem`.

use files::testing::MemoryFileSystem;
use import::ConflictPolicy;
use support::{DESTINATION, add, already_imported, copy_to, planned, resolve_all};

mod support;

/// spec: IMPORT-003.10
#[test]
fn targets_that_differ_only_in_case() {
    let file_system = MemoryFileSystem::new();
    let files = [
        planned(&file_system, "a/dscf0001.jpg", b"first", "dscf0001.jpg"),
        planned(&file_system, "b/DSCF0001.JPG", b"other", "DSCF0001.JPG"),
    ];

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(
        outcomes,
        [copy_to("dscf0001.jpg"), copy_to("DSCF0001-1.JPG")]
    );
}

/// spec: IMPORT-003.14
#[test]
fn destination_names_are_compared_ignoring_case() {
    let file_system = MemoryFileSystem::new();
    let files = [
        planned(&file_system, "DSCF0001.JPG", b"same", "DSCF0001.JPG"),
        planned(&file_system, "DSCF0002.JPG", b"ours", "ÄRGER.JPG"),
    ];
    add(&file_system, DESTINATION, "dscf0001.jpg", b"same");
    add(&file_system, DESTINATION, "ärger.jpg", b"theirs");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(
        outcomes,
        [already_imported("dscf0001.jpg"), copy_to("ÄRGER-1.JPG")]
    );
}

const PRECOMPOSED: &str = "Caf\u{e9}.JPG";
const DECOMPOSED: &str = "Cafe\u{301}.JPG";

/// spec: IMPORT-003.12
#[test]
fn identical_file_stored_decomposed_is_already_imported() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(&file_system, "DSCF0001.JPG", b"same", PRECOMPOSED)];
    add(&file_system, DESTINATION, DECOMPOSED, b"same");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [already_imported(DECOMPOSED)]);
}

/// spec: IMPORT-003.12
#[test]
fn different_file_stored_decomposed_gets_a_suffix() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(&file_system, "DSCF0001.JPG", b"ours", PRECOMPOSED)];
    add(&file_system, DESTINATION, DECOMPOSED, b"theirs");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [copy_to("Caf\u{e9}-1.JPG")]);
}

/// spec: IMPORT-003.13
#[test]
fn targets_that_differ_only_in_normalization_form() {
    let file_system = MemoryFileSystem::new();
    let files = [
        planned(&file_system, "a/Caf\u{e9}.JPG", b"first", PRECOMPOSED),
        planned(&file_system, "b/Caf\u{e9}.JPG", b"other", DECOMPOSED),
    ];

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(
        outcomes,
        [copy_to(PRECOMPOSED), copy_to("Cafe\u{301}-1.JPG")]
    );
}

/// spec: IMPORT-003.15
#[test]
fn identical_to_the_precomposed_one_of_two_names_is_already_imported() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(&file_system, "DSCF0001.JPG", b"same", DECOMPOSED)];
    add(&file_system, DESTINATION, DECOMPOSED, b"theirs");
    add(&file_system, DESTINATION, PRECOMPOSED, b"same");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [already_imported(PRECOMPOSED)]);
}

/// spec: IMPORT-003.15
#[test]
fn identical_to_the_decomposed_one_of_two_names_is_already_imported() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(&file_system, "DSCF0001.JPG", b"same", PRECOMPOSED)];
    add(&file_system, DESTINATION, PRECOMPOSED, b"theirs");
    add(&file_system, DESTINATION, DECOMPOSED, b"same");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [already_imported(DECOMPOSED)]);
}

/// spec: IMPORT-003.15
#[test]
fn identical_to_both_names_reports_the_exact_spelling() {
    for target in [PRECOMPOSED, DECOMPOSED] {
        let file_system = MemoryFileSystem::new();
        let files = [planned(&file_system, "DSCF0001.JPG", b"same", target)];
        add(&file_system, DESTINATION, PRECOMPOSED, b"same");
        add(&file_system, DESTINATION, DECOMPOSED, b"same");

        let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

        assert_eq!(outcomes, [already_imported(target)]);
    }
}

/// spec: IMPORT-003.15
#[test]
fn different_from_both_names_gets_a_suffix() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(&file_system, "DSCF0001.JPG", b"ours", PRECOMPOSED)];
    add(&file_system, DESTINATION, PRECOMPOSED, b"theirs");
    add(&file_system, DESTINATION, DECOMPOSED, b"others");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [copy_to("Caf\u{e9}-1.JPG")]);
}

const TARGET_IN_FOLDER: &str = "Caf\u{e9}/DSCF0001.JPG";
const IN_UPPER_DECOMPOSED_FOLDER: &str = "CAFE\u{301}/DSCF0001.JPG";
const IN_LOWER_FOLDER: &str = "caf\u{e9}/DSCF0001.JPG";

/// spec: IMPORT-003.16
#[test]
fn identical_file_in_a_folder_of_another_spelling_is_already_imported() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(
        &file_system,
        "DSCF0001.JPG",
        b"same",
        TARGET_IN_FOLDER,
    )];
    add(
        &file_system,
        DESTINATION,
        IN_UPPER_DECOMPOSED_FOLDER,
        b"same",
    );

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [already_imported(IN_UPPER_DECOMPOSED_FOLDER)]);
}

/// spec: IMPORT-003.16
#[test]
fn different_file_in_a_folder_of_another_spelling_gets_a_suffix() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(
        &file_system,
        "DSCF0001.JPG",
        b"ours",
        TARGET_IN_FOLDER,
    )];
    add(
        &file_system,
        DESTINATION,
        IN_UPPER_DECOMPOSED_FOLDER,
        b"theirs",
    );

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [copy_to("Caf\u{e9}/DSCF0001-1.JPG")]);
}

/// spec: IMPORT-003.16
#[test]
fn every_folder_of_another_spelling_is_compared() {
    let file_system = MemoryFileSystem::new();
    let files = [
        planned(&file_system, "a/DSCF0001.JPG", b"same", TARGET_IN_FOLDER),
        planned(&file_system, "b/DSCF0001.JPG", b"ours", TARGET_IN_FOLDER),
    ];
    add(
        &file_system,
        DESTINATION,
        IN_UPPER_DECOMPOSED_FOLDER,
        b"theirs",
    );
    add(&file_system, DESTINATION, IN_LOWER_FOLDER, b"same");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(
        outcomes,
        [
            already_imported(IN_LOWER_FOLDER),
            copy_to("Caf\u{e9}/DSCF0001-1.JPG")
        ]
    );
}
