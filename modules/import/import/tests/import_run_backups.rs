// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import run copies each file to its backup destinations (IMPORT-007), against
//! `MemoryFileSystem`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod backup_support;

use domain::Timestamp;
use files::testing::Fault;
use files::{CopyErrorKind, CopyStep};
use import::{BackupOutcome, FileError, FileOutcome};
use std::path::Path;

use backup_support::{
    BACKUP, BY_YEAR, DESTINATION, FLAT, SECOND_BACKUP, Setup, at, backup_copy_error, copied_target,
    modified, only_backup, only_file, part, paths_below, source_with,
};

/// spec: IMPORT-007.1
#[test]
fn a_copy_goes_to_every_backup_destination() {
    let file_system = source_with(&[("DSCF0001.RAF", b"raw content")]);

    let summary = Setup::new(BY_YEAR, &[BACKUP, SECOND_BACKUP]).on(&file_system);

    let file = only_file(&summary);
    assert_eq!(copied_target(file), "2026/DSCF0001.RAF");
    for root in [DESTINATION, BACKUP, SECOND_BACKUP] {
        let target = at(root, "2026/DSCF0001.RAF");
        assert_eq!(file_system.content(&target).unwrap(), b"raw content");
        assert_eq!(file_system.modified(&target), Some(modified()));
    }
    let destinations: Vec<&Path> = file
        .backups
        .iter()
        .map(|backup| backup.destination.as_path())
        .collect();
    assert_eq!(destinations, [Path::new(BACKUP), Path::new(SECOND_BACKUP)]);
    assert!(
        file.backups
            .iter()
            .all(|backup| matches!(backup.outcome, BackupOutcome::BackedUp))
    );
}

/// spec: IMPORT-007.2
#[test]
fn the_backup_keeps_the_resolved_name() {
    let file_system = source_with(&[("DSCF0001.RAF", b"new photo")]);
    file_system.add_file(&at(DESTINATION, "DSCF0001.RAF"), b"old photo!", modified());
    file_system.add_folder(Path::new(BACKUP));

    let summary = Setup::new(FLAT, &[BACKUP]).on(&file_system);

    let file = only_file(&summary);
    assert_eq!(copied_target(file), "DSCF0001-1.RAF");
    assert!(matches!(only_backup(file), BackupOutcome::BackedUp));
    assert_eq!(
        file_system.content(&at(BACKUP, "DSCF0001-1.RAF")).unwrap(),
        b"new photo"
    );
    assert_eq!(
        paths_below(&file_system, BACKUP),
        [at(BACKUP, "DSCF0001-1.RAF")]
    );
}

/// spec: IMPORT-007.4
#[test]
fn each_backup_is_verified_against_the_source() {
    let file_system = source_with(&[("DSCF0001.RAF", b"raw content")]);
    let backup_target = at(BACKUP, "DSCF0001.RAF");
    file_system.inject_fault(&backup_target, Fault::ReadReturnsDifferentBytes);

    let summary = Setup::new(FLAT, &[BACKUP]).on(&file_system);

    let file = only_file(&summary);
    assert_eq!(copied_target(file), "DSCF0001.RAF");
    let error = backup_copy_error(only_backup(file));
    assert_eq!(error.step, CopyStep::Verify);
    let CopyErrorKind::ChecksumMismatch {
        source_checksum,
        target_checksum,
    } = &error.kind
    else {
        panic!("a checksum mismatch expected, got {:?}", error.kind);
    };
    assert_ne!(source_checksum, target_checksum);
    assert_eq!(file_system.content(&backup_target), None);
}

/// spec: IMPORT-007.5
#[test]
fn a_failed_backup_does_not_undo_the_import() {
    let file_system = source_with(&[("DSCF0001.RAF", b"raw content")]);
    file_system.inject_fault(
        &part(BACKUP, "DSCF0001.RAF"),
        Fault::WriteFails { after_bytes: 3 },
    );

    let summary = Setup::new(FLAT, &[BACKUP]).on(&file_system);

    let file = only_file(&summary);
    assert_eq!(copied_target(file), "DSCF0001.RAF");
    assert_eq!(
        file_system
            .content(&at(DESTINATION, "DSCF0001.RAF"))
            .unwrap(),
        b"raw content"
    );
    let error = backup_copy_error(only_backup(file));
    assert_eq!(error.step, CopyStep::WritePart);
    assert!(matches!(error.kind, CopyErrorKind::Io { .. }));
    assert_eq!(file_system.content(&at(BACKUP, "DSCF0001.RAF")), None);
    assert_eq!(file_system.content(&part(BACKUP, "DSCF0001.RAF")), None);
}

/// spec: IMPORT-007.6
#[test]
fn a_failed_import_destination_keeps_verified_backups() {
    let file_system = source_with(&[("DSCF0001.RAF", b"raw content")]);
    file_system.inject_fault(
        &part(DESTINATION, "DSCF0001.RAF"),
        Fault::WriteFails { after_bytes: 3 },
    );

    let summary = Setup::new(FLAT, &[BACKUP]).on(&file_system);

    let file = only_file(&summary);
    assert!(matches!(
        file.outcome,
        FileOutcome::Failed(FileError::Copy(_))
    ));
    assert!(matches!(only_backup(file), BackupOutcome::BackedUp));
    assert_eq!(
        file_system.content(&at(BACKUP, "DSCF0001.RAF")).unwrap(),
        b"raw content"
    );
    assert_eq!(file_system.content(&at(DESTINATION, "DSCF0001.RAF")), None);
    assert_eq!(
        file_system.content(&part(DESTINATION, "DSCF0001.RAF")),
        None
    );
    assert!(summary.has_failures());
}

/// spec: IMPORT-007.7
#[test]
fn a_file_already_imported_is_backed_up() {
    let file_system = source_with(&[("DSCF0001.RAF", b"raw content")]);
    let imported = at(DESTINATION, "DSCF0001.RAF");
    file_system.add_file(&imported, b"raw content", Timestamp::UNIX_EPOCH);

    let summary = Setup::new(FLAT, &[BACKUP]).on(&file_system);

    let file = only_file(&summary);
    assert!(matches!(file.outcome, FileOutcome::AlreadyImported { .. }));
    assert_eq!(file_system.modified(&imported), Some(Timestamp::UNIX_EPOCH));
    assert!(matches!(only_backup(file), BackupOutcome::BackedUp));
    let backup_target = at(BACKUP, "DSCF0001.RAF");
    assert_eq!(file_system.content(&backup_target).unwrap(), b"raw content");
    assert_eq!(file_system.modified(&backup_target), Some(modified()));
}

#[test]
fn without_backup_destinations_nothing_is_backed_up() {
    let file_system = source_with(&[("DSCF0001.RAF", b"raw content")]);

    let summary = Setup::new(FLAT, &[]).on(&file_system);

    assert!(only_file(&summary).backups.is_empty());
    assert!(summary.backups.is_empty());
}
