// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the import run finds at backup targets and backup destinations, and what it
//! counts (IMPORT-007), against `MemoryFileSystem`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod backup_support;

use domain::Timestamp;
use files::testing::Fault;
use import::{
    BackupCounts, BackupDestinationProblem, BackupError, BackupOutcome, BackupReport,
    ConflictPolicy, FileOutcome, RunError,
};
use std::path::{Path, PathBuf};

use backup_support::{
    BACKUP, DESTINATION, FLAT, SECOND_BACKUP, Setup, at, copied_target, modified, only_backup,
    only_file, part, paths_below, source_with,
};

/// spec: IMPORT-007.8
#[test]
fn identical_content_at_the_backup_target_is_kept() {
    let file_system = source_with(&[("DSCF0001.RAF", b"raw content")]);
    let backup_target = at(BACKUP, "DSCF0001.RAF");
    file_system.add_file(&backup_target, b"raw content", Timestamp::UNIX_EPOCH);

    let summary = Setup::new(FLAT, &[BACKUP]).on(&file_system);

    let file = only_file(&summary);
    assert_eq!(copied_target(file), "DSCF0001.RAF");
    assert!(matches!(only_backup(file), BackupOutcome::AlreadyBackedUp));
    assert_eq!(
        file_system.modified(&backup_target),
        Some(Timestamp::UNIX_EPOCH)
    );
    assert_eq!(paths_below(&file_system, BACKUP), [backup_target]);
}

/// spec: IMPORT-007.9
#[test]
fn other_content_at_the_backup_target_is_never_overwritten() {
    // The same size, so the content is compared, and another size.
    for other in [&b"raw CONTENT"[..], b"other"] {
        let file_system = source_with(&[("DSCF0001.RAF", b"raw content")]);
        let backup_target = at(BACKUP, "DSCF0001.RAF");
        file_system.add_file(&backup_target, other, Timestamp::UNIX_EPOCH);

        let summary = Setup::new(FLAT, &[BACKUP]).on(&file_system);

        let file = only_file(&summary);
        assert_eq!(copied_target(file), "DSCF0001.RAF");
        let BackupOutcome::Failed(BackupError::NameConflict { path }) = only_backup(file) else {
            panic!("a name conflict expected, got {:?}", file.backups);
        };
        assert_eq!(path, &backup_target);
        assert_eq!(file_system.content(&backup_target).unwrap(), other);
        assert_eq!(paths_below(&file_system, BACKUP), [backup_target]);
    }
}

/// spec: IMPORT-007.10
#[test]
fn a_name_conflict_in_the_import_destination_is_not_backed_up() {
    let file_system = source_with(&[("DSCF0001.RAF", b"new photo")]);
    file_system.add_file(&at(DESTINATION, "DSCF0001.RAF"), b"old photo!", modified());
    file_system.add_folder(Path::new(BACKUP));

    let summary = Setup::new(FLAT, &[BACKUP])
        .with_policy(ConflictPolicy::Skip)
        .on(&file_system);

    let file = only_file(&summary);
    assert!(matches!(
        file.outcome,
        FileOutcome::SkippedNameConflict { .. }
    ));
    assert!(file.backups.is_empty());
    assert!(paths_below(&file_system, BACKUP).is_empty());
}

/// spec: IMPORT-007.11
#[test]
fn invalid_backup_destinations_stop_the_run_before_it_starts() {
    let cases: [(&[&str], &str, BackupDestinationProblem); 6] = [
        (
            &[DESTINATION],
            DESTINATION,
            BackupDestinationProblem::IsImportDestination,
        ),
        (
            &["/library/"],
            "/library/",
            BackupDestinationProblem::IsImportDestination,
        ),
        (
            &["/library/backup"],
            "/library/backup",
            BackupDestinationProblem::InsideImportDestination,
        ),
        (
            &["/"],
            "/",
            BackupDestinationProblem::ContainsImportDestination,
        ),
        (
            &["/media/card/backup"],
            "/media/card/backup",
            BackupDestinationProblem::InsideSource,
        ),
        (
            &[BACKUP, SECOND_BACKUP, BACKUP],
            BACKUP,
            BackupDestinationProblem::GivenTwice,
        ),
    ];
    for (backups, named, problem) in cases {
        let file_system = source_with(&[("DSCF0001.RAF", b"raw content")]);
        let before = file_system.paths();

        let error = Setup::new(FLAT, backups).try_on(&file_system).unwrap_err();

        let RunError::BackupDestination(error) = error else {
            panic!("an invalid backup destination expected for {backups:?}, got {error:?}");
        };
        assert_eq!(error.path, Path::new(named));
        assert_eq!(error.problem, problem, "{backups:?}");
        assert!(error.to_string().contains(named));
        assert_eq!(file_system.paths(), before);
    }
}

/// spec: IMPORT-007.12
#[test]
fn a_missing_backup_destination_is_created() {
    let file_system = source_with(&[("DSCF0001.RAF", b"raw content")]);
    let backup = "/mnt/new/backup";

    let summary = Setup::new(FLAT, &[backup]).on(&file_system);

    let file = only_file(&summary);
    let [BackupReport { outcome, .. }] = &file.backups[..] else {
        panic!("one backup expected");
    };
    assert!(matches!(outcome, BackupOutcome::BackedUp));
    assert_eq!(
        file_system.content(&at(backup, "DSCF0001.RAF")).unwrap(),
        b"raw content"
    );
}

/// spec: IMPORT-007.13
#[test]
fn the_summary_counts_each_backup_destination() {
    let file_system = source_with(&[
        ("a.RAF", b"first"),
        ("b.RAF", b"second"),
        ("c.RAF", b"third"),
    ]);
    file_system.add_file(&at(BACKUP, "b.RAF"), b"second", Timestamp::UNIX_EPOCH);
    file_system.inject_fault(&part(BACKUP, "c.RAF"), Fault::WriteFails { after_bytes: 0 });

    let summary = Setup::new(FLAT, &[BACKUP]).on(&file_system);

    assert_eq!(
        summary.backups,
        [BackupCounts {
            destination: PathBuf::from(BACKUP),
            backed_up: 1,
            already_backed_up: 1,
            backed_up_before: 0,
            failed: 1,
            receives_photos: true,
        }]
    );
}
