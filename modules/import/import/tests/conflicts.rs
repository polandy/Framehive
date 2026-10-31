// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Name conflicts and files already imported (IMPORT-003), against
//! `MemoryFileSystem`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::{Path, PathBuf};

use domain::Timestamp;
use files::testing::{Fault, FnvHasher, MemoryFileSystem};
use import::{ConflictOutcome, ConflictPolicy, TargetRequest, TargetResolver};

const SOURCE: &str = "/media/card";
const DESTINATION: &str = "/library";

const POLICIES: [ConflictPolicy; 3] = [
    ConflictPolicy::AddSuffix,
    ConflictPolicy::Skip,
    ConflictPolicy::Fail,
];

fn add(file_system: &MemoryFileSystem, root: &str, relative: &str, content: &[u8]) {
    file_system.add_file(
        &Path::new(root).join(relative),
        content,
        Timestamp::UNIX_EPOCH,
    );
}

/// A source file and its planned target.
struct Planned {
    source_path: &'static str,
    target: &'static str,
    size: u64,
}

fn planned(
    file_system: &MemoryFileSystem,
    source_path: &'static str,
    content: &[u8],
    target: &'static str,
) -> Planned {
    add(file_system, SOURCE, source_path, content);
    Planned {
        source_path,
        target,
        size: content.len() as u64,
    }
}

fn resolve_all(
    file_system: &MemoryFileSystem,
    policy: ConflictPolicy,
    files: &[Planned],
) -> Vec<ConflictOutcome> {
    let mut resolver = TargetResolver::new(
        file_system,
        &FnvHasher,
        Path::new(SOURCE),
        Path::new(DESTINATION),
        policy,
    );
    files
        .iter()
        .map(|file| {
            resolver
                .resolve(&TargetRequest {
                    source_path: Path::new(file.source_path),
                    size: file.size,
                    target: file.target,
                })
                .unwrap()
        })
        .collect()
}

fn copy_to(target: &str) -> ConflictOutcome {
    ConflictOutcome::CopyTo {
        target: target.to_owned(),
    }
}

fn already_imported(target: &str) -> ConflictOutcome {
    ConflictOutcome::AlreadyImported {
        target: target.to_owned(),
    }
}

/// spec: IMPORT-003.1
#[test]
fn running_the_same_import_again_copies_nothing() {
    for policy in POLICIES {
        let file_system = MemoryFileSystem::new();
        let files = [
            planned(&file_system, "DSCF0001.RAF", b"raw 1", "2026/DSCF0001.RAF"),
            planned(&file_system, "DSCF0002.JPG", b"jpeg 2", "2026/DSCF0002.JPG"),
        ];
        add(&file_system, DESTINATION, "2026/DSCF0001.RAF", b"raw 1");
        add(&file_system, DESTINATION, "2026/DSCF0002.JPG", b"jpeg 2");

        let outcomes = resolve_all(&file_system, policy, &files);

        assert_eq!(
            outcomes,
            [
                already_imported("2026/DSCF0001.RAF"),
                already_imported("2026/DSCF0002.JPG"),
            ],
            "{policy:?}"
        );
        assert_eq!(outcomes[0].to_string(), "already imported");
    }
}

/// spec: IMPORT-003.2
#[test]
fn suffix_for_a_different_file() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(
        &file_system,
        "DSCF0001.RAF",
        b"ours",
        "2026/DSCF0001.RAF",
    )];
    add(&file_system, DESTINATION, "2026/DSCF0001.RAF", b"other");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [copy_to("2026/DSCF0001-1.RAF")]);
}

/// spec: IMPORT-003.3
#[test]
fn first_free_suffix() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(
        &file_system,
        "DSCF0001.RAF",
        b"ours",
        "DSCF0001.RAF",
    )];
    add(&file_system, DESTINATION, "DSCF0001.RAF", b"other");
    add(&file_system, DESTINATION, "DSCF0001-1.RAF", b"another");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [copy_to("DSCF0001-2.RAF")]);
}

#[test]
fn suffix_without_an_extension_goes_at_the_end() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(
        &file_system,
        "DSCF0001.RAF",
        b"ours",
        "2026/DSCF0001",
    )];
    add(&file_system, DESTINATION, "2026/DSCF0001", b"other");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [copy_to("2026/DSCF0001-1")]);
}

/// spec: IMPORT-003.4
#[test]
fn rerun_after_a_suffix_copies_nothing() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(
        &file_system,
        "DSCF0001.RAF",
        b"ours",
        "DSCF0001.RAF",
    )];
    add(&file_system, DESTINATION, "DSCF0001.RAF", b"othr");
    add(&file_system, DESTINATION, "DSCF0001-1.RAF", b"ours");

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [already_imported("DSCF0001-1.RAF")]);
}

/// spec: IMPORT-003.5
#[test]
fn skip() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(
        &file_system,
        "DSCF0001.RAF",
        b"ours",
        "DSCF0001.RAF",
    )];
    add(&file_system, DESTINATION, "DSCF0001.RAF", b"other");
    let target = Path::new(DESTINATION).join("DSCF0001.RAF");

    let outcomes = resolve_all(&file_system, ConflictPolicy::Skip, &files);

    assert_eq!(
        outcomes,
        [ConflictOutcome::SkippedNameConflict {
            target: "DSCF0001.RAF".to_owned()
        }]
    );
    assert_eq!(outcomes[0].to_string(), "skipped: name conflict");
    assert_eq!(file_system.content(&target).unwrap(), b"other");
}

/// spec: IMPORT-003.6
#[test]
fn fail() {
    let file_system = MemoryFileSystem::new();
    let files = [
        planned(&file_system, "DSCF0001.RAF", b"ours", "DSCF0001.RAF"),
        planned(&file_system, "DSCF0002.RAF", b"next", "DSCF0002.RAF"),
    ];
    add(&file_system, DESTINATION, "DSCF0001.RAF", b"other");
    let target = Path::new(DESTINATION).join("DSCF0001.RAF");

    let outcomes = resolve_all(&file_system, ConflictPolicy::Fail, &files);

    assert_eq!(
        outcomes,
        [
            ConflictOutcome::FailedNameConflict {
                target: "DSCF0001.RAF".to_owned()
            },
            copy_to("DSCF0002.RAF"),
        ]
    );
    assert_eq!(outcomes[0].to_string(), "failed: name conflict");
    assert_eq!(file_system.content(&target).unwrap(), b"other");
}

/// spec: IMPORT-003.7
#[test]
fn folder_at_the_target_path() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(
        &file_system,
        "DSCF0001.RAF",
        b"ours",
        "2026/DSCF0001.RAF",
    )];
    let folder = Path::new(DESTINATION).join("2026/DSCF0001.RAF");
    file_system.add_folder(&folder);
    let paths_before = file_system.paths();

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [copy_to("2026/DSCF0001-1.RAF")]);
    assert_eq!(file_system.paths(), paths_before);
}

/// spec: IMPORT-003.8
#[test]
fn two_files_of_one_plan_with_the_same_target() {
    let file_system = MemoryFileSystem::new();
    let files = [
        planned(
            &file_system,
            "a/DSCF0001.JPG",
            b"first",
            "2026/DSCF0001.JPG",
        ),
        planned(
            &file_system,
            "b/DSCF0001.JPG",
            b"other",
            "2026/DSCF0001.JPG",
        ),
    ];

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(
        outcomes,
        [copy_to("2026/DSCF0001.JPG"), copy_to("2026/DSCF0001-1.JPG")]
    );
}

/// spec: IMPORT-003.9
#[test]
fn same_photo_twice_in_one_source() {
    for policy in POLICIES {
        let file_system = MemoryFileSystem::new();
        let files = [
            planned(&file_system, "slot1/DSCF0001.RAF", b"raw", "DSCF0001.RAF"),
            planned(&file_system, "slot2/DSCF0001.RAF", b"raw", "DSCF0001.RAF"),
        ];

        let outcomes = resolve_all(&file_system, policy, &files);

        assert_eq!(
            outcomes,
            [
                copy_to("DSCF0001.RAF"),
                ConflictOutcome::DuplicateOf {
                    earlier: PathBuf::from("slot1/DSCF0001.RAF")
                },
            ],
            "{policy:?}"
        );
        assert_eq!(outcomes[1].to_string(), "duplicate of slot1/DSCF0001.RAF");
    }
}

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

/// spec: IMPORT-003.11
#[test]
fn sizes_decide_first() {
    let file_system = MemoryFileSystem::new();
    let files = [planned(
        &file_system,
        "DSCF0001.RAF",
        b"ours",
        "DSCF0001.RAF",
    )];
    add(&file_system, DESTINATION, "DSCF0001.RAF", b"longer");
    // Reading either file fails, so a checksum would turn into an error.
    let fault = Fault::ReadFails { after_bytes: 0 };
    file_system.inject_fault(&Path::new(SOURCE).join("DSCF0001.RAF"), fault.clone());
    file_system.inject_fault(&Path::new(DESTINATION).join("DSCF0001.RAF"), fault);

    let outcomes = resolve_all(&file_system, ConflictPolicy::AddSuffix, &files);

    assert_eq!(outcomes, [copy_to("DSCF0001-1.RAF")]);
}

#[test]
fn unreadable_file_to_compare_is_an_error() {
    let file_system = MemoryFileSystem::new();
    add(&file_system, SOURCE, "DSCF0001.RAF", b"ours");
    add(&file_system, DESTINATION, "DSCF0001.RAF", b"them");
    let target = Path::new(DESTINATION).join("DSCF0001.RAF");
    file_system.inject_fault(&target, Fault::ReadFails { after_bytes: 0 });
    let mut resolver = TargetResolver::new(
        &file_system,
        &FnvHasher,
        Path::new(SOURCE),
        Path::new(DESTINATION),
        ConflictPolicy::AddSuffix,
    );

    let error = resolver
        .resolve(&TargetRequest {
            source_path: Path::new("DSCF0001.RAF"),
            size: 4,
            target: "DSCF0001.RAF",
        })
        .unwrap_err();

    assert_eq!(error.path, target);
    assert!(error.to_string().contains("/library/DSCF0001.RAF"));
}
