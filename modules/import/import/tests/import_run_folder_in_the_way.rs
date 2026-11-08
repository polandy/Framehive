// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A file in the way of a target folder in a dry run and in a run (IMPORT-017),
//! against `MemoryFileSystem`, `MemoryFreeSpace` and `MemoryImportHistory`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers may panic on bad fixtures"
)]

mod history_support;

use files::testing::{MemoryFileSystem, MemoryFreeSpace};
use import::{ConflictPolicy, FileOutcome, ImportSummary};

use history_support::{
    BY_DATE, DESTINATION, FLAT, Import, Machine, add, at, paths_below, photo, report, source_with,
};

const PHOTO: &str = "DSCF0001.RAF";
const PHOTO_JPEG: &str = "DSCF0001.JPG";

/// The reason a file failed, or a panic naming its outcome.
fn failure(summary: &ImportSummary, path: &str) -> String {
    match &report(summary, path).outcome {
        FileOutcome::Failed(error) => error.to_string(),
        outcome => panic!("{path} did not fail: {outcome:?}"),
    }
}

/// spec: IMPORT-017.4
#[test]
fn the_dry_run_and_the_run_report_a_file_in_the_way_alike() {
    let file_system = source_with(&[(PHOTO, &photo(1))]);
    add(&file_system, DESTINATION, "2026-10-17", b"a file");
    let paths_before = paths_below(&file_system, DESTINATION);
    let reason = format!(
        "the target folder {} is a file",
        at(DESTINATION, "2026-10-17").display()
    );

    let dry_run = Machine::new().import(&file_system, &Import::with(BY_DATE).dry_running());
    let run = Machine::new().import(&file_system, &Import::with(BY_DATE));

    assert_eq!(failure(&dry_run, PHOTO), reason);
    assert_eq!(failure(&run, PHOTO), reason);
    assert_eq!(paths_below(&file_system, DESTINATION), paths_before);
}

/// spec: IMPORT-017.6
#[test]
fn a_file_in_the_way_needs_no_free_space() {
    let file_system = source_with(&[(PHOTO, &photo(1))]);
    add(&file_system, DESTINATION, "2026-10-17", b"a file");
    let mut machine = Machine::new();
    machine.free_space = MemoryFreeSpace::new().with_file_system("/", 1);

    let summary = machine
        .try_import(
            &file_system,
            &Import::with(BY_DATE),
            &mut import::NoProgress,
        )
        .unwrap();

    assert!(failure(&summary, PHOTO).ends_with("is a file"));
}

/// The source of IMPORT-017.5: a photo whose target is the video destination, and a
/// video.
fn photo_named_like_the_videos() -> MemoryFileSystem {
    source_with(&[(PHOTO_JPEG, &photo(1)), ("clip.MOV", &[2; 50])])
}

fn into_videos(policy: ConflictPolicy) -> Import {
    Import::with(FLAT)
        .with_video_destination("/library/DSCF0001.JPG")
        .with_policy(policy)
}

/// spec: IMPORT-017.5
#[test]
fn a_photo_does_not_take_a_folder_of_the_video_destination() {
    let dry_run = Machine::new().import(
        &photo_named_like_the_videos(),
        &into_videos(ConflictPolicy::AddSuffix).dry_running(),
    );
    let run = Machine::new().import(
        &photo_named_like_the_videos(),
        &into_videos(ConflictPolicy::AddSuffix),
    );

    assert!(matches!(
        &report(&dry_run, PHOTO_JPEG).outcome,
        FileOutcome::WouldCopy { target } if target == "DSCF0001-1.JPG"
    ));
    assert!(matches!(
        &report(&run, PHOTO_JPEG).outcome,
        FileOutcome::Copied { target, .. } if target == "DSCF0001-1.JPG"
    ));
}

/// spec: IMPORT-017.5
#[test]
fn with_skip_a_photo_at_a_folder_of_the_video_destination_is_skipped() {
    for dry_run in [true, false] {
        let mut import = into_videos(ConflictPolicy::Skip);
        if dry_run {
            import = import.dry_running();
        }

        let summary = Machine::new().import(&photo_named_like_the_videos(), &import);

        assert!(
            matches!(
                &report(&summary, PHOTO_JPEG).outcome,
                FileOutcome::SkippedNameConflict { target } if target == PHOTO_JPEG
            ),
            "dry run: {dry_run}"
        );
    }
}
