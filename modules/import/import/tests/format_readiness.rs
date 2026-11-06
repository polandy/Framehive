// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The format readiness of a finished import run, evaluated from its summary
//! (IMPORT-008).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::{Path, PathBuf};

use domain::Checksum;
use import::{
    BackupCounts, BackupError, BackupOutcome, BackupReport, FileError, FileOutcome, FileReport,
    FormatBlocker, FormatReadiness, ImportSummary, LeftOut, LeftOutReason,
};

const BACKUP: &str = "/backup";
const SECOND_BACKUP: &str = "/second-backup";

fn copied() -> FileOutcome {
    FileOutcome::Copied {
        target: "2026/DSCF0001.RAF".to_owned(),
        checksum: Checksum::new(Checksum::BLAKE3, vec![7; 32]).unwrap(),
    }
}

fn failed() -> FileOutcome {
    FileOutcome::Failed(FileError::NameNotUtf8)
}

fn backed_up(destination: &str) -> BackupReport {
    backup(destination, BackupOutcome::BackedUp)
}

fn backup(destination: &str, outcome: BackupOutcome) -> BackupReport {
    BackupReport {
        destination: PathBuf::from(destination),
        outcome,
    }
}

fn backup_failed(destination: &str) -> BackupReport {
    let error = BackupError::NameConflict {
        path: PathBuf::from(destination).join("DSCF0001.RAF"),
    };
    backup(destination, BackupOutcome::Failed(error))
}

fn file(path: &str, outcome: FileOutcome, backups: Vec<BackupReport>) -> FileReport {
    FileReport {
        path: PathBuf::from(path),
        dated_by_modification_time: None,
        outcome,
        backups,
        in_video_destination: false,
        // Set by `summary`, which knows the backup destinations of the run.
        required_locations: 0,
    }
}

fn left_out(path: &str, reason: LeftOutReason) -> LeftOut {
    LeftOut {
        path: PathBuf::from(path),
        reason,
    }
}

/// A summary of a finished run with `files`, backed up to `backup_destinations`;
/// each file is required at the import destination and every backup destination.
fn summary(files: Vec<FileReport>, backup_destinations: &[&str]) -> ImportSummary {
    let files = files
        .into_iter()
        .map(|file| FileReport {
            required_locations: 1 + backup_destinations.len(),
            ..file
        })
        .collect();
    ImportSummary {
        files,
        backups: backup_destinations
            .iter()
            .map(|destination| BackupCounts {
                destination: PathBuf::from(destination),
                ..BackupCounts::default()
            })
            .collect(),
        ..ImportSummary::default()
    }
}

fn not_saved(path: &str, verified_at: usize, required_at: usize) -> FormatBlocker {
    FormatBlocker::NotSaved {
        path: PathBuf::from(path),
        verified_at,
        required_at,
    }
}

fn all_copied() -> Vec<FileReport> {
    vec![file("DSCF0001.RAF", copied(), Vec::new())]
}

/// spec: IMPORT-008.1
#[test]
fn every_file_copied_and_backed_up_can_be_formatted() {
    let files = vec![
        file("DSCF0001.RAF", copied(), vec![backed_up(BACKUP)]),
        file("DSCF0002.RAF", copied(), vec![backed_up(BACKUP)]),
    ];

    let readiness = FormatReadiness::of(&summary(files, &[BACKUP]));

    assert!(readiness.can_be_formatted());
    assert_eq!(readiness.required_locations, 2);
    assert!(readiness.blockers.is_empty());
}

/// spec: IMPORT-008.2
#[test]
fn already_imported_and_already_backed_up_is_saved() {
    let already = FileOutcome::AlreadyImported {
        target: "2026/DSCF0001.RAF".to_owned(),
    };
    let backups = vec![backup(BACKUP, BackupOutcome::AlreadyBackedUp)];
    let files = vec![file("DSCF0001.RAF", already, backups)];

    let readiness = FormatReadiness::of(&summary(files, &[BACKUP]));

    assert!(readiness.can_be_formatted());
}

/// spec: IMPORT-008.3
#[test]
fn without_backup_destinations_one_location_is_required() {
    let readiness = FormatReadiness::of(&summary(all_copied(), &[]));

    assert!(readiness.can_be_formatted());
    assert_eq!(readiness.required_locations, 1);
}

/// spec: IMPORT-008.4
#[test]
fn a_failed_backup_keeps_the_card() {
    let backups = vec![backed_up(BACKUP), backup_failed(SECOND_BACKUP)];
    let files = vec![file("DSCF0001.RAF", copied(), backups)];

    let readiness = FormatReadiness::of(&summary(files, &[BACKUP, SECOND_BACKUP]));

    assert!(!readiness.can_be_formatted());
    assert_eq!(readiness.required_locations, 3);
    assert_eq!(readiness.blockers, vec![not_saved("DSCF0001.RAF", 2, 3)]);
}

/// spec: IMPORT-008.5
#[test]
fn a_failed_import_destination_keeps_the_card() {
    let files = vec![file("DSCF0001.RAF", failed(), vec![backed_up(BACKUP)])];

    let readiness = FormatReadiness::of(&summary(files, &[BACKUP]));

    assert!(!readiness.can_be_formatted());
    assert_eq!(readiness.required_locations, 2);
    assert_eq!(readiness.blockers, vec![not_saved("DSCF0001.RAF", 1, 2)]);
}

/// spec: IMPORT-008.6
#[test]
fn a_skipped_file_keeps_the_card() {
    let skipped = FileOutcome::SkippedNameConflict {
        target: "2026/DSCF0001.RAF".to_owned(),
    };
    let files = vec![file("DSCF0001.RAF", skipped, Vec::new())];

    let readiness = FormatReadiness::of(&summary(files, &[]));

    assert!(!readiness.can_be_formatted());
    assert_eq!(readiness.required_locations, 1);
    assert_eq!(readiness.blockers, vec![not_saved("DSCF0001.RAF", 0, 1)]);
}

fn with_duplicate(earlier_outcome: FileOutcome) -> ImportSummary {
    let duplicate = FileOutcome::DuplicateOf {
        earlier: PathBuf::from("A/DSCF0001.RAF"),
    };
    let files = vec![
        file("A/DSCF0001.RAF", earlier_outcome, Vec::new()),
        file("B/DSCF0001.RAF", duplicate, Vec::new()),
    ];
    summary(files, &[])
}

/// spec: IMPORT-008.7
#[test]
fn a_duplicate_of_a_copied_file_is_saved() {
    let readiness = FormatReadiness::of(&with_duplicate(copied()));

    assert!(readiness.can_be_formatted());
}

/// spec: IMPORT-008.7
#[test]
fn a_duplicate_of_a_failed_file_is_not_saved() {
    let readiness = FormatReadiness::of(&with_duplicate(failed()));

    assert!(!readiness.can_be_formatted());
    assert_eq!(
        readiness.blockers,
        vec![
            not_saved("A/DSCF0001.RAF", 0, 1),
            not_saved("B/DSCF0001.RAF", 0, 1)
        ]
    );
}

fn blocked_by_left_out(reason: LeftOutReason, path: &str) {
    let mut run = summary(all_copied(), &[]);
    run.left_out = vec![left_out(path, reason.clone())];

    let readiness = FormatReadiness::of(&run);

    assert!(!readiness.can_be_formatted());
    let expected = FormatBlocker::LeftOut {
        path: Path::new(path).to_path_buf(),
        reason,
    };
    assert_eq!(readiness.blockers, vec![expected]);
    assert_eq!(readiness.left_out_not_saved, 0);
}

/// spec: IMPORT-008.8
#[test]
fn an_unreadable_folder_keeps_the_card() {
    let reason = LeftOutReason::CannotBeRead("permission denied".to_owned());

    blocked_by_left_out(reason, "DCIM/101_FUJI");
}

/// spec: IMPORT-008.9
#[test]
fn a_sidecar_left_out_keeps_the_card() {
    blocked_by_left_out(LeftOutReason::SidecarNeedsGrouping, "DSCF0001.xmp");
}

/// spec: IMPORT-008.10
#[test]
fn camera_system_files_do_not_keep_the_card() {
    let mut run = summary(all_copied(), &[]);
    run.left_out = vec![
        left_out("MISC/CAMERA.DAT", LeftOutReason::UnsupportedType),
        left_out(".Trashes", LeftOutReason::Hidden),
        left_out("LINK.RAF", LeftOutReason::SymbolicLink),
    ];

    let readiness = FormatReadiness::of(&run);

    assert!(readiness.can_be_formatted());
    assert_eq!(readiness.left_out_not_saved, 3);
}

/// spec: IMPORT-008.11
#[test]
fn a_stopped_run_keeps_the_card() {
    // The second file was never started, so it is not in the summary.
    let mut run = summary(all_copied(), &[]);
    run.stopped = true;

    let readiness = FormatReadiness::of(&run);

    assert!(!readiness.can_be_formatted());
    assert!(readiness.stopped);
}

/// spec: IMPORT-008.12
#[test]
fn a_dry_run_keeps_the_card() {
    let would_copy = FileOutcome::WouldCopy {
        target: "2026/DSCF0001.RAF".to_owned(),
    };
    let mut run = summary(vec![file("DSCF0001.RAF", would_copy, Vec::new())], &[]);
    run.dry_run = true;

    let readiness = FormatReadiness::of(&run);

    assert!(!readiness.can_be_formatted());
    assert!(readiness.dry_run);
    assert_eq!(readiness.blockers, vec![not_saved("DSCF0001.RAF", 0, 1)]);
}

/// spec: IMPORT-008.12
#[test]
fn a_dry_run_of_files_already_imported_keeps_the_card() {
    let already = FileOutcome::AlreadyImported {
        target: "2026/DSCF0001.RAF".to_owned(),
    };
    let mut run = summary(vec![file("DSCF0001.RAF", already, Vec::new())], &[]);
    run.dry_run = true;

    let readiness = FormatReadiness::of(&run);

    assert!(!readiness.can_be_formatted());
    assert!(readiness.dry_run);
    assert!(readiness.blockers.is_empty());
}

/// spec: IMPORT-008.8
/// spec: IMPORT-008.10
#[test]
fn only_left_out_entries_that_do_not_keep_the_card_are_counted() {
    let mut run = summary(all_copied(), &[]);
    run.left_out = vec![
        left_out(
            "DCIM/101_FUJI",
            LeftOutReason::CannotBeRead("denied".to_owned()),
        ),
        left_out(".Trashes", LeftOutReason::Hidden),
    ];

    let readiness = FormatReadiness::of(&run);

    assert!(!readiness.can_be_formatted());
    assert_eq!(readiness.left_out_not_saved, 1);
    assert_eq!(readiness.blockers.len(), 1);
}

/// spec: IMPORT-008.13
#[test]
fn an_empty_source_can_be_formatted() {
    let readiness = FormatReadiness::of(&summary(Vec::new(), &[]));

    assert!(readiness.can_be_formatted());
    assert_eq!(readiness.left_out_not_saved, 0);
}

/// spec: IMPORT-013.3
#[test]
fn each_unsaved_file_carries_the_locations_it_requires() {
    let photo = file("DSCF0001.RAF", copied(), vec![backed_up(BACKUP)]);
    let video = FileReport {
        required_locations: 3,
        ..file("clip.MOV", copied(), vec![backed_up(SECOND_BACKUP)])
    };
    let mut run = summary(vec![photo], &[BACKUP]);
    run.files.push(video);

    let readiness = FormatReadiness::of(&run);

    assert!(!readiness.can_be_formatted());
    assert_eq!(readiness.required_locations, 3);
    assert!(readiness.required_locations_differ);
    assert_eq!(readiness.blockers, vec![not_saved("clip.MOV", 2, 3)]);
    let photos_only = FormatReadiness::of(&summary(all_copied(), &[BACKUP]));
    assert!(!photos_only.required_locations_differ);
}
