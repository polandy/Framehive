// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Video destinations and video backup destinations of an import run (IMPORT-013),
//! against `MemoryFileSystem`, `MemoryFreeSpace` and `MemoryImportHistory`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers may panic on bad fixtures"
)]

mod history_support;

use std::path::PathBuf;

use files::testing::{MemoryFileSystem, MemoryFreeSpace};
use history::ImportHistory;
use import::{
    BackupOutcome, ConflictPolicy, FileOutcome, FormatReadiness, ImportSummary, RunError,
    SpaceShortage,
};

use history_support::{
    BACKUP, DESTINATION, FLAT, Import, Machine, add, at, paths_below, report, source_with,
};

const PHOTO: &str = "DSCF0001.RAF";
const VIDEO: &str = "clip.MOV";
const VIDEOS: &str = "/videos";
const VIDEO_BACKUP: &str = "/video-backup";

/// The source of IMPORT-013.1: a RAW file of 300 bytes and a video of 500 bytes.
fn photo_and_video() -> MemoryFileSystem {
    source_with(&[(PHOTO, &[1; 300]), (VIDEO, &[2; 500])])
}

fn paths(paths: &[(&str, &str)]) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|(root, relative)| at(root, relative))
        .collect()
}

fn backed_up_at(summary: &ImportSummary, path: &str) -> Vec<PathBuf> {
    report(summary, path)
        .backups
        .iter()
        .filter(|backup| matches!(backup.outcome, BackupOutcome::BackedUp))
        .map(|backup| backup.destination.clone())
        .collect()
}

/// spec: IMPORT-013.1
#[test]
fn videos_go_to_the_video_destination() {
    let file_system = photo_and_video();

    let summary = Machine::new().import(
        &file_system,
        &Import::with(FLAT).with_video_destination(VIDEOS),
    );

    assert_eq!(
        paths_below(&file_system, DESTINATION),
        paths(&[(DESTINATION, PHOTO)])
    );
    assert_eq!(paths_below(&file_system, VIDEOS), paths(&[(VIDEOS, VIDEO)]));
    assert!(report(&summary, VIDEO).in_video_destination);
    assert!(!report(&summary, PHOTO).in_video_destination);
}

/// spec: IMPORT-013.2
#[test]
fn without_a_video_destination_videos_stay_with_photos() {
    let file_system = photo_and_video();

    Machine::new().import(&file_system, &Import::with(FLAT));

    assert_eq!(
        paths_below(&file_system, DESTINATION),
        paths(&[(DESTINATION, PHOTO), (DESTINATION, VIDEO)])
    );
}

/// spec: IMPORT-013.3
#[test]
fn videos_are_backed_up_to_the_video_backup_destinations() {
    let file_system = photo_and_video();

    let summary = Machine::new().import(
        &file_system,
        &Import::with(FLAT)
            .with_backups(&[BACKUP])
            .with_video_backups(&[VIDEO_BACKUP]),
    );

    assert_eq!(paths_below(&file_system, BACKUP), paths(&[(BACKUP, PHOTO)]));
    assert_eq!(
        paths_below(&file_system, VIDEO_BACKUP),
        paths(&[(VIDEO_BACKUP, VIDEO)])
    );
    assert_eq!(backed_up_at(&summary, PHOTO), [PathBuf::from(BACKUP)]);
    assert_eq!(backed_up_at(&summary, VIDEO), [PathBuf::from(VIDEO_BACKUP)]);
    let counts: Vec<(PathBuf, usize)> = summary
        .backups
        .iter()
        .map(|counts| (counts.destination.clone(), counts.backed_up))
        .collect();
    assert_eq!(
        counts,
        [(PathBuf::from(BACKUP), 1), (PathBuf::from(VIDEO_BACKUP), 1)]
    );
}

/// spec: IMPORT-013.3
/// spec: IMPORT-008.15
#[test]
fn a_folder_may_be_a_backup_and_a_video_backup_destination() {
    let file_system = photo_and_video();

    let summary = Machine::new().import(
        &file_system,
        &Import::with(FLAT)
            .with_backups(&[BACKUP])
            .with_video_backups(&[BACKUP, VIDEO_BACKUP]),
    );

    assert_eq!(
        paths_below(&file_system, BACKUP),
        paths(&[(BACKUP, PHOTO), (BACKUP, VIDEO)])
    );
    assert_eq!(summary.backups.len(), 2);
    assert_eq!(summary.backups[0].backed_up, 2);
    assert!(summary.backups[0].receives_photos);
    assert!(!summary.backups[1].receives_photos);
}

/// spec: IMPORT-013.4
#[test]
fn without_video_backup_destinations_videos_use_the_backup_destinations() {
    let file_system = photo_and_video();

    Machine::new().import(
        &file_system,
        &Import::with(FLAT)
            .with_video_destination(VIDEOS)
            .with_backups(&[BACKUP]),
    );

    assert_eq!(paths_below(&file_system, VIDEOS), paths(&[(VIDEOS, VIDEO)]));
    assert_eq!(
        paths_below(&file_system, BACKUP),
        paths(&[(BACKUP, PHOTO), (BACKUP, VIDEO)])
    );
}

/// spec: IMPORT-013.5
#[test]
fn a_name_conflict_at_the_video_destination_is_resolved_there() {
    let file_system = photo_and_video();
    add(&file_system, VIDEOS, VIDEO, b"another clip");

    let summary = Machine::new().import(
        &file_system,
        &Import::with(FLAT)
            .with_video_destination(VIDEOS)
            .with_policy(ConflictPolicy::AddSuffix),
    );

    assert!(matches!(
        &report(&summary, VIDEO).outcome,
        FileOutcome::Copied { target, .. } if target == "clip-1.MOV"
    ));
    assert!(file_system.paths().contains(&at(VIDEOS, "clip-1.MOV")));
    assert_eq!(
        paths_below(&file_system, DESTINATION),
        paths(&[(DESTINATION, PHOTO)])
    );
}

/// spec: IMPORT-013.6
#[test]
fn a_video_imported_before_at_the_video_destination_is_recognized() {
    let file_system = photo_and_video();
    let machine = Machine::new();
    let import = Import::with(FLAT).with_video_destination(VIDEOS);
    machine.import(&file_system, &import);
    assert_eq!(paths_below(&file_system, VIDEOS), paths(&[(VIDEOS, VIDEO)]));
    let before = file_system.paths();

    let summary = machine.import(&file_system, &import);

    assert!(matches!(
        &report(&summary, VIDEO).outcome,
        FileOutcome::ImportedBefore { target } if target == VIDEO
    ));
    assert_eq!(file_system.paths(), before);
}

/// spec: IMPORT-013.6
#[test]
fn a_video_imported_to_the_import_destination_is_copied_to_a_new_video_destination() {
    let file_system = photo_and_video();
    let machine = Machine::new();
    machine.import(&file_system, &Import::with(FLAT));

    let summary = machine.import(
        &file_system,
        &Import::with(FLAT).with_video_destination(VIDEOS),
    );

    assert!(matches!(
        report(&summary, VIDEO).outcome,
        FileOutcome::Copied { .. }
    ));
    assert!(matches!(
        report(&summary, PHOTO).outcome,
        FileOutcome::ImportedBefore { .. }
    ));
}

/// spec: IMPORT-013.7
#[test]
fn free_space_counts_videos_at_the_video_destination() {
    let file_system = photo_and_video();
    let mut machine = Machine::new();
    machine.free_space = MemoryFreeSpace::new()
        .with_file_system("/", 1_000_000)
        .with_file_system(VIDEOS, 499);

    let result = machine.try_import(
        &file_system,
        &Import::with(FLAT).with_video_destination(VIDEOS),
        &mut import::NoProgress,
    );

    let Err(RunError::NotEnoughSpace(error)) = result else {
        panic!("not enough space expected, got {result:?}");
    };
    assert_eq!(
        error.shortages,
        [SpaceShortage {
            destinations: vec![PathBuf::from(VIDEOS)],
            bytes_to_write: 500,
            available_bytes: 499,
        }]
    );
    assert!(paths_below(&file_system, DESTINATION).is_empty());
}

/// spec: IMPORT-013.9
#[test]
fn a_missing_video_destination_inside_the_import_destination_is_created() {
    let file_system = photo_and_video();
    let videos = "/library/Videos";

    Machine::new().import(
        &file_system,
        &Import::with(FLAT).with_video_destination(videos),
    );

    assert_eq!(
        paths_below(&file_system, DESTINATION),
        paths(&[
            (DESTINATION, PHOTO),
            (DESTINATION, "Videos"),
            (videos, VIDEO)
        ])
    );
}

/// spec: IMPORT-013.9
#[test]
fn the_video_destination_may_be_the_import_destination() {
    let file_system = photo_and_video();

    Machine::new().import(
        &file_system,
        &Import::with(FLAT).with_video_destination(DESTINATION),
    );

    assert_eq!(
        paths_below(&file_system, DESTINATION),
        paths(&[(DESTINATION, PHOTO), (DESTINATION, VIDEO)])
    );
}

/// The video destinations the history lists for the runs of `machine`, oldest first.
fn recorded_video_destinations(machine: &Machine) -> Vec<Option<PathBuf>> {
    machine
        .history
        .list_runs()
        .unwrap()
        .into_iter()
        .map(|run| run.start.video_destination)
        .collect()
}

/// spec: IMPORT-013.10
#[test]
fn the_run_records_its_video_destination() {
    let file_system = photo_and_video();
    let machine = Machine::new();

    machine.import(
        &file_system,
        &Import::with(FLAT).with_video_destination(VIDEOS),
    );
    machine.import(&file_system, &Import::with(FLAT));

    assert_eq!(
        recorded_video_destinations(&machine),
        [Some(PathBuf::from(VIDEOS)), None]
    );
}

/// A video destination that is the import destination is not a folder of its own, so
/// the run records none.
/// spec: IMPORT-013.10
#[test]
fn a_video_destination_that_is_the_import_destination_is_not_recorded() {
    let file_system = photo_and_video();
    let machine = Machine::new();

    machine.import(
        &file_system,
        &Import::with(FLAT).with_video_destination(DESTINATION),
    );

    assert_eq!(recorded_video_destinations(&machine), [None]);
}

/// spec: IMPORT-013.3
#[test]
fn format_readiness_requires_each_file_at_its_own_destinations() {
    let file_system = photo_and_video();

    let summary = Machine::new().import(
        &file_system,
        &Import::with(FLAT)
            .with_video_destination(VIDEOS)
            .with_backups(&[BACKUP])
            .with_video_backups(&[VIDEO_BACKUP, "/video-backup-2"]),
    );

    let readiness = FormatReadiness::of(&summary);
    assert!(readiness.can_be_formatted(), "{readiness:?}");
}
