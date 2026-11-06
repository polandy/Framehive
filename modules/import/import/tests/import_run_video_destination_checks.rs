// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The checks of the video destination and the video backup destinations before an
//! import run starts (IMPORT-013, "Before the run"), against `MemoryFileSystem`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers may panic on bad fixtures"
)]

mod history_support;

use std::path::{Path, PathBuf};

use files::testing::MemoryFileSystem;
use import::{BackupDestinationProblem, RunError, VideoDestinationProblem};

use history_support::{FLAT, Import, Machine, source_with};

const VIDEOS: &str = "/videos";
const VIDEO_BACKUP: &str = "/video-backup";

/// The source of IMPORT-013.1.
fn photo_and_video() -> MemoryFileSystem {
    source_with(&[("DSCF0001.RAF", &[1; 300]), ("clip.MOV", &[2; 500])])
}

/// A run with `import` that must not start; returns its error.
fn refused(import: &Import) -> RunError {
    let file_system = photo_and_video();
    let before = file_system.paths();
    let result = Machine::new().try_import(&file_system, import, &mut import::NoProgress);
    assert_eq!(file_system.paths(), before, "nothing may be written");
    match result {
        Err(error) => error,
        Ok(summary) => panic!("the run must not start, got {summary:?}"),
    }
}

/// The folder and the problem of an invalid backup or video backup destination.
fn backup_problem(error: &RunError) -> (&str, PathBuf, BackupDestinationProblem) {
    match error {
        RunError::BackupDestination(error) => ("backup", error.path.clone(), error.problem),
        RunError::VideoBackupDestination(error) => {
            ("video backup", error.path.clone(), error.problem)
        }
        other => panic!("an invalid backup destination expected, got {other:?}"),
    }
}

/// spec: IMPORT-013.8
#[test]
fn a_video_destination_inside_the_source_stops_the_run() {
    let inside = "/media/card/videos";

    let error = refused(&Import::with(FLAT).with_video_destination(inside));

    let RunError::VideoDestination(error) = &error else {
        panic!("an invalid video destination expected, got {error:?}");
    };
    assert_eq!(error.path, Path::new(inside));
    assert_eq!(error.problem, VideoDestinationProblem::InsideSource);
    assert!(error.to_string().contains(inside));
}

/// spec: IMPORT-013.8
#[test]
fn a_video_destination_containing_the_source_stops_the_run() {
    let error = refused(&Import::with(FLAT).with_video_destination("/media"));

    let RunError::VideoDestination(error) = &error else {
        panic!("an invalid video destination expected, got {error:?}");
    };
    assert_eq!(error.problem, VideoDestinationProblem::ContainsSource);
}

/// Asserts that `import` does not start because its `kind` (`backup` or `video
/// backup`) destination `folder` has `problem`, and that the reason names the folder.
fn assert_refused(import: &Import, kind: &str, folder: &str, problem: BackupDestinationProblem) {
    let error = refused(import);

    assert_eq!(
        backup_problem(&error),
        (kind, PathBuf::from(folder), problem),
        "{error}"
    );
    assert!(error.to_string().contains(folder), "{error}");
}

fn with_videos() -> Import {
    Import::with(FLAT).with_video_destination(VIDEOS)
}

/// spec: IMPORT-013.8
#[test]
fn a_video_backup_destination_that_is_or_lies_inside_the_video_destination_stops_the_run() {
    use BackupDestinationProblem::{InsideVideoDestination, IsVideoDestination};

    assert_refused(
        &with_videos().with_video_backups(&[VIDEOS]),
        "video backup",
        VIDEOS,
        IsVideoDestination,
    );
    assert_refused(
        &with_videos().with_video_backups(&["/videos/backup"]),
        "video backup",
        "/videos/backup",
        InsideVideoDestination,
    );
}

/// spec: IMPORT-013.8
#[test]
fn a_video_backup_destination_at_the_import_destination_or_the_source_stops_the_run() {
    use BackupDestinationProblem::{
        ContainsImportDestination, InsideImportDestination, InsideSource,
    };

    assert_refused(
        &with_videos().with_video_backups(&["/library/video-backup"]),
        "video backup",
        "/library/video-backup",
        InsideImportDestination,
    );
    assert_refused(
        &with_videos().with_video_backups(&["/"]),
        "video backup",
        "/",
        ContainsImportDestination,
    );
    assert_refused(
        &Import::with(FLAT).with_video_backups(&["/media/card/backup"]),
        "video backup",
        "/media/card/backup",
        InsideSource,
    );
}

/// spec: IMPORT-013.8
#[test]
fn a_backup_destination_inside_the_video_destination_stops_the_run() {
    assert_refused(
        &with_videos().with_backups(&["/videos/backup"]),
        "backup",
        "/videos/backup",
        BackupDestinationProblem::InsideVideoDestination,
    );
}

/// spec: IMPORT-013.8
#[test]
fn a_video_backup_destination_given_twice_stops_the_run() {
    assert_refused(
        &with_videos().with_video_backups(&[VIDEO_BACKUP, VIDEO_BACKUP]),
        "video backup",
        VIDEO_BACKUP,
        BackupDestinationProblem::GivenTwice,
    );
}
