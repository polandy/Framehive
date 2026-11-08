// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A carried-along source sidecar in the format readiness (IMPORT-020, IMPORT-008).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::PathBuf;

use domain::Checksum;
use import::{
    BackupCounts, BackupError, BackupOutcome, BackupReport, CarriedSidecar, FileOutcome,
    FileReport, FormatBlocker, FormatReadiness, ImportSummary, LeftOut, LeftOutReason,
};

const BACKUPS: [&str; 2] = ["/backup-1", "/backup-2"];

/// `DSCF0042.RAF`, copied and backed up to both backups, with `DSCF0042.RAF.xmp`
/// verified at `sidecar_verified_at` of its three locations.
fn raw_with_sidecar(sidecar_verified_at: usize) -> FileReport {
    FileReport {
        path: PathBuf::from("DSCF0042.RAF"),
        dated_by_modification_time: None,
        outcome: FileOutcome::Copied {
            target: "20261017-0042.RAF".to_owned(),
            checksum: Checksum::new(Checksum::BLAKE3, vec![7; 32]).unwrap(),
        },
        backups: BACKUPS
            .iter()
            .map(|destination| BackupReport {
                destination: PathBuf::from(destination),
                outcome: BackupOutcome::BackedUp,
            })
            .collect(),
        in_video_destination: false,
        required_locations: 3,
        sidecar: Some("20261017-0042.RAF.xmp".to_owned()),
        carried_sidecars: vec![CarriedSidecar {
            path: PathBuf::from("DSCF0042.RAF.xmp"),
            target: Some("20261017-0042.RAF.xmp".to_owned()),
            verified_at: sidecar_verified_at,
        }],
        existing_values: Vec::new(),
    }
}

fn summary(files: Vec<FileReport>) -> ImportSummary {
    ImportSummary {
        files,
        backups: BACKUPS
            .iter()
            .map(|destination| BackupCounts {
                destination: PathBuf::from(destination),
                receives_photos: true,
                ..BackupCounts::default()
            })
            .collect(),
        ..ImportSummary::default()
    }
}

/// spec: IMPORT-020.1
#[test]
fn a_carried_sidecar_saved_everywhere_lets_the_card_be_formatted() {
    let readiness = FormatReadiness::of(&summary(vec![raw_with_sidecar(3)]));

    assert!(readiness.can_be_formatted());
}

/// spec: IMPORT-020.8
#[test]
fn a_carried_sidecar_not_saved_keeps_the_card() {
    // The backup took the file, but its sidecar failed there.
    let mut file = raw_with_sidecar(1);
    file.required_locations = 2;
    file.backups.truncate(1);
    let mut run = summary(vec![file]);
    run.backups.truncate(1);

    let readiness = FormatReadiness::of(&run);

    assert!(!readiness.can_be_formatted());
    assert_eq!(readiness.required_locations, 2);
    assert_eq!(
        readiness.blockers,
        [FormatBlocker::NotSaved {
            path: PathBuf::from("DSCF0042.RAF.xmp"),
            verified_at: 1,
            required_at: 2,
        }]
    );
}

/// spec: IMPORT-020.3
#[test]
fn a_sidecar_without_its_file_keeps_the_card() {
    let mut run = summary(vec![raw_with_sidecar(3)]);
    run.left_out = vec![LeftOut {
        path: PathBuf::from("DSCF0043.xmp"),
        reason: LeftOutReason::SidecarWithoutItsFile,
    }];

    let readiness = FormatReadiness::of(&run);

    assert!(!readiness.can_be_formatted());
    assert_eq!(
        readiness.blockers,
        [FormatBlocker::LeftOut {
            path: PathBuf::from("DSCF0043.xmp"),
            reason: LeftOutReason::SidecarWithoutItsFile,
        }]
    );
}

#[test]
fn a_failed_backup_counts_against_the_sidecar_too() {
    let mut file = raw_with_sidecar(2);
    let error = BackupError::NameConflict {
        path: PathBuf::from("/backup-2/20261017-0042.RAF"),
    };
    file.backups[1].outcome = BackupOutcome::Failed(error);

    let readiness = FormatReadiness::of(&summary(vec![file]));

    let paths: Vec<_> = readiness
        .blockers
        .iter()
        .map(|blocker| match blocker {
            FormatBlocker::NotSaved {
                path, verified_at, ..
            } => (path.to_str().unwrap().to_owned(), *verified_at),
            FormatBlocker::LeftOut { .. } => panic!("no left-out entry expected"),
        })
        .collect();
    assert_eq!(
        paths,
        [
            ("DSCF0042.RAF".to_owned(), 2),
            ("DSCF0042.RAF.xmp".to_owned(), 2)
        ]
    );
}
