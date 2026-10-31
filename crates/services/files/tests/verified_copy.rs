// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The verified copy of one file (IMPORT-004), against `MemoryFileSystem`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::{Path, PathBuf};

use domain::{Checksum, Timestamp};
use files::testing::{Fault, FnvHasher, MemoryFileSystem};
use files::{CopyError, CopyErrorKind, CopyStep, EntryKind, FileSystem, Hasher, copy_verified};

const SOURCE: &str = "/card/DCIM/100_FUJI/DSCF0001.RAF";
const DESTINATION: &str = "/photos";
/// 2026-10-17 14:05:09 UTC
const MODIFIED_SECONDS: i64 = 1_792_245_909;

struct Import {
    file_system: MemoryFileSystem,
    content: Vec<u8>,
}

impl Import {
    /// A source file of several copy buffers, so a copy can stop half-way, and a
    /// durable destination folder.
    fn new() -> Self {
        let file_system = MemoryFileSystem::new();
        let content: Vec<u8> = (0..=250).cycle().take(200_003).collect();
        file_system.add_file(Path::new(SOURCE), &content, modified());
        file_system.add_folder(Path::new(DESTINATION));
        Import {
            file_system,
            content,
        }
    }

    fn copy_to(&self, target: &Path) -> Result<Checksum, CopyError> {
        copy_verified(&self.file_system, &FnvHasher, Path::new(SOURCE), target)
    }

    fn entry(&self, path: &Path) -> Option<EntryKind> {
        self.file_system.entry(path).unwrap()
    }
}

fn modified() -> Timestamp {
    Timestamp::from_unix_seconds(MODIFIED_SECONDS)
}

fn destination(relative: &str) -> PathBuf {
    Path::new(DESTINATION).join(relative)
}

fn part(target: &Path) -> PathBuf {
    let mut name = target.as_os_str().to_owned();
    name.push(".part");
    PathBuf::from(name)
}

fn checksum_of(content: &[u8]) -> Checksum {
    let mut hash = FnvHasher.start();
    hash.update(content);
    hash.finish()
}

fn source_paths(file_system: &MemoryFileSystem) -> Vec<PathBuf> {
    let paths = file_system.paths();
    paths
        .into_iter()
        .filter(|path| path.starts_with("/card"))
        .collect()
}

/// spec: IMPORT-004.1
#[test]
fn copy_matches_the_source_and_reports_its_checksum() {
    let import = Import::new();
    let target = destination("2026/DSCF0001.RAF");
    import.file_system.add_folder(&destination("2026"));

    let checksum = import.copy_to(&target).unwrap();

    assert_eq!(import.file_system.content(&target).unwrap(), import.content);
    assert_eq!(checksum, checksum_of(&import.content));
    assert_eq!(checksum.algorithm(), FnvHasher::ALGORITHM);
    assert_eq!(import.entry(&part(&target)), None);
}

/// spec: IMPORT-004.2
#[test]
fn source_is_left_unchanged() {
    let import = Import::new();
    let before = source_paths(&import.file_system);

    import.copy_to(&destination("2026/DSCF0001.RAF")).unwrap();

    assert_eq!(source_paths(&import.file_system), before);
    let source = Path::new(SOURCE);
    assert_eq!(import.file_system.content(source).unwrap(), import.content);
    assert_eq!(import.file_system.modified(source).unwrap(), modified());
}

/// spec: IMPORT-004.3
#[test]
fn missing_folders_are_created() {
    let import = Import::new();
    let target = destination("2026/2026-10-17/DSCF0001.RAF");
    assert_eq!(import.entry(&destination("2026")), None);

    import.copy_to(&target).unwrap();

    assert_eq!(
        import.entry(&destination("2026/2026-10-17")),
        Some(EntryKind::Folder)
    );
    assert_eq!(import.file_system.content(&target).unwrap(), import.content);
}

/// spec: IMPORT-004.4
#[test]
fn read_error_leaves_nothing_behind() {
    let import = Import::new();
    let source = Path::new(SOURCE);
    import.file_system.inject_fault(
        source,
        Fault::ReadFails {
            after_bytes: 100_000,
        },
    );
    let target = destination("2026/DSCF0001.RAF");

    let error = import.copy_to(&target).unwrap_err();

    assert_eq!(error.step, CopyStep::ReadSource);
    assert!(
        matches!(&error.kind, CopyErrorKind::Io { path, .. } if path == source),
        "{error}"
    );
    assert!(error.cleanup.is_none(), "{error}");
    assert_eq!(import.entry(&target), None);
    assert_eq!(import.entry(&part(&target)), None);
}

/// spec: IMPORT-004.5
#[test]
fn write_error_leaves_nothing_behind() {
    let import = Import::new();
    let target = destination("2026/DSCF0001.RAF");
    let full_device = Fault::WriteFails {
        after_bytes: 70_000,
    };
    import.file_system.inject_fault(&part(&target), full_device);

    let error = import.copy_to(&target).unwrap_err();

    assert_eq!(error.step, CopyStep::WritePart);
    assert!(
        matches!(&error.kind, CopyErrorKind::Io { error, .. } if error.kind() == std::io::ErrorKind::StorageFull),
        "{error}"
    );
    assert!(error.to_string().contains("no space left"), "{error}");
    assert_eq!(import.entry(&target), None);
    assert_eq!(import.entry(&part(&target)), None);
}

/// spec: IMPORT-004.6
#[test]
fn bad_copy_is_detected_and_removed() {
    let import = Import::new();
    let target = destination("2026/DSCF0001.RAF");
    import
        .file_system
        .inject_fault(&target, Fault::ReadReturnsDifferentBytes);

    let error = import.copy_to(&target).unwrap_err();

    assert_eq!(error.step, CopyStep::Verify);
    let CopyErrorKind::ChecksumMismatch {
        source_checksum,
        target_checksum,
    } = &error.kind
    else {
        panic!("expected a checksum mismatch, got {error}");
    };
    let mut read_back = import.content.clone();
    if let Some(last) = read_back.last_mut() {
        *last = !*last;
    }
    assert_eq!(source_checksum, &checksum_of(&import.content));
    assert_eq!(target_checksum, &checksum_of(&read_back));
    let message = error.to_string();
    assert!(message.contains(&source_checksum.to_string()), "{message}");
    assert!(message.contains(&target_checksum.to_string()), "{message}");
    assert_eq!(import.entry(&target), None);
    assert_eq!(import.entry(&part(&target)), None);
}

/// spec: IMPORT-004.7
#[test]
fn copied_file_survives_a_crash() {
    let import = Import::new();
    let target = destination("2026/2026-10-17/DSCF0001.RAF");
    import.copy_to(&target).unwrap();

    import.file_system.crash();

    assert_eq!(import.file_system.content(&target).unwrap(), import.content);
    assert_eq!(import.file_system.modified(&target).unwrap(), modified());
}

/// spec: IMPORT-004.8
#[test]
fn no_partial_file_under_the_target_name_at_any_stop() {
    let target = destination("2026/2026-10-17/DSCF0001.RAF");
    let complete = Import::new();
    let start = complete.file_system.operations();
    complete.copy_to(&target).unwrap();
    let operations = complete.file_system.operations() - start;

    for stop in 0..=operations {
        for crash in [false, true] {
            let import = Import::new();
            import.file_system.stop_after_operations(stop);
            let result = import.copy_to(&target);
            if crash {
                import.file_system.crash();
            }

            let content = import.file_system.content(&target);
            assert!(
                content.is_none() || content.as_deref() == Some(import.content.as_slice()),
                "stop after {stop} operations, crash {crash}: the target has partial content"
            );
            if result.is_ok() {
                assert_eq!(
                    content.unwrap(),
                    import.content,
                    "stop after {stop}, crash {crash}"
                );
            }
        }
    }
}

/// spec: IMPORT-004.9
#[test]
fn file_that_appears_at_the_target_is_kept() {
    let import = Import::new();
    let target = destination("2026/DSCF0001.RAF");
    let other_program = Fault::OtherProgramCreates {
        path: target.clone(),
        content: b"another program's file".to_vec(),
    };
    import
        .file_system
        .inject_fault(&part(&target), other_program);

    let error = import.copy_to(&target).unwrap_err();

    assert_eq!(error.step, CopyStep::Rename);
    assert!(
        matches!(&error.kind, CopyErrorKind::TargetExists { path } if path == &target),
        "{error}"
    );
    assert_eq!(
        import.file_system.content(&target).unwrap(),
        b"another program's file"
    );
    assert_eq!(import.entry(&part(&target)), None);
}

/// spec: IMPORT-004.10
#[test]
fn leftover_part_file_is_replaced() {
    let import = Import::new();
    let target = destination("2026/DSCF0001.RAF");
    let leftover = vec![7; 300_000];
    import
        .file_system
        .add_file(&part(&target), &leftover, Timestamp::UNIX_EPOCH);

    import.copy_to(&target).unwrap();

    assert_eq!(import.file_system.content(&target).unwrap(), import.content);
    assert_eq!(import.entry(&part(&target)), None);
}

/// spec: IMPORT-004.12
#[test]
fn target_keeps_the_modification_time() {
    let import = Import::new();
    let target = destination("2026/DSCF0001.RAF");

    import.copy_to(&target).unwrap();

    assert_eq!(import.file_system.modified(&target).unwrap(), modified());
}

#[test]
fn target_is_removed_when_it_cannot_be_read_back() {
    let import = Import::new();
    let target = destination("2026/DSCF0001.RAF");
    import
        .file_system
        .inject_fault(&target, Fault::ReadFails { after_bytes: 10 });

    let error = import.copy_to(&target).unwrap_err();

    assert_eq!(error.step, CopyStep::ReadBack);
    assert_eq!(import.entry(&target), None);
    assert_eq!(import.entry(&part(&target)), None);
}

#[test]
fn missing_source_creates_no_part_file() {
    let import = Import::new();
    let target = destination("DSCF0002.RAF");

    let error = copy_verified(
        &import.file_system,
        &FnvHasher,
        Path::new("/card/missing.RAF"),
        &target,
    )
    .unwrap_err();

    assert_eq!(error.step, CopyStep::ReadSource);
    assert_eq!(import.entry(&part(&target)), None);
}

#[test]
fn file_left_behind_is_reported() {
    let import = Import::new();
    let target = destination("2026/DSCF0001.RAF");
    import.file_system.add_folder(&destination("2026"));
    // The program stops right after the .part file was created and one buffer was
    // read: the copy fails, and removing the .part file fails too.
    import.file_system.stop_after_operations(5);

    let error = import.copy_to(&target).unwrap_err();

    let cleanup = error
        .cleanup
        .as_ref()
        .expect("the .part file cannot be removed");
    assert_eq!(cleanup.path, part(&target));
    assert!(
        error.to_string().contains("could not be removed"),
        "{error}"
    );
}
