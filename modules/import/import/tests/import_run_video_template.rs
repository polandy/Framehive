// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The video name template of an import run (IMPORT-018), against `MemoryFileSystem`,
//! `MemorySequenceCounters` and `MemoryImportHistory`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers may panic on bad fixtures"
)]

mod history_support;

use std::path::PathBuf;

use import::{MissingJobCode, MixedSequenceKinds, NoProgress, RunError};
use naming::{CounterDate, SequenceError, SequenceKind, StoredCounters};

use history_support::{DESTINATION, FLAT, Import, Machine, at, paths_below, source_with};

const PHOTO: &str = "DSCF0001.RAF";
const VIDEO: &str = "clip.MOV";
const VIDEOS: &str = "/videos";

/// The source of IMPORT-018.1; both files carry the same time, so `DSCF0001` sorts
/// first by its base name.
fn photo_and_video() -> files::testing::MemoryFileSystem {
    source_with(&[(PHOTO, &[1; 300]), (VIDEO, &[2; 500])])
}

fn paths(root: &str, relatives: &[&str]) -> Vec<PathBuf> {
    relatives
        .iter()
        .map(|relative| at(root, relative))
        .collect()
}

/// spec: IMPORT-018.1
#[test]
fn a_run_names_its_videos_by_the_video_name_template() {
    let file_system = photo_and_video();

    Machine::new().import(
        &file_system,
        &Import::with(FLAT).with_video_template("Clips/{name}.{ext}"),
    );

    assert_eq!(
        paths_below(&file_system, DESTINATION),
        paths(DESTINATION, &["Clips", "Clips/clip.MOV", PHOTO])
    );
}

/// spec: IMPORT-018.3
#[test]
fn the_video_name_template_is_read_below_the_video_destination() {
    let file_system = photo_and_video();

    Machine::new().import(
        &file_system,
        &Import::with(FLAT)
            .with_video_destination(VIDEOS)
            .with_video_template("{date:%Y}/{name}.{ext}"),
    );

    assert_eq!(
        paths_below(&file_system, VIDEOS),
        paths(VIDEOS, &["2026", "2026/clip.MOV"])
    );
    assert_eq!(
        paths_below(&file_system, DESTINATION),
        paths(DESTINATION, &[PHOTO])
    );
}

/// spec: IMPORT-018.4
#[test]
fn photos_and_videos_share_one_numbering() {
    let file_system = photo_and_video();
    let machine = Machine::new();
    machine.counters().set(StoredCounters {
        stored_last: 100,
        ..StoredCounters::default()
    });

    machine.import(
        &file_system,
        &Import::with("{seq_stored:3}-{name}.{ext}").with_video_template("V{seq_stored:4}.{ext}"),
    );

    assert_eq!(
        paths_below(&file_system, DESTINATION),
        paths(DESTINATION, &["101-DSCF0001.RAF", "V0102.MOV"])
    );
    assert_eq!(machine.stored_last(), 102);
}

/// spec: IMPORT-018.5
#[test]
fn different_sequence_kinds_keep_the_run_from_starting() {
    let file_system = photo_and_video();
    let paths_before = file_system.paths();
    let machine = Machine::new();
    let counters = StoredCounters {
        imported_today_date: CounterDate::parse("2026-10-31"),
        imported_today_last: 7,
        stored_last: 40,
    };
    machine.counters().set(counters);

    let error = machine
        .try_import(
            &file_system,
            &Import::with("{seq_today:3}.{ext}").with_video_template("{seq_stored:3}.{ext}"),
            &mut NoProgress,
        )
        .unwrap_err();

    assert!(
        matches!(
            error,
            RunError::MixedSequenceKinds(MixedSequenceKinds {
                video: SequenceKind::Stored,
                name: SequenceKind::ImportedToday,
            })
        ),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "the video template uses {seq_stored} but the name template uses {seq_today}; \
         use one sequence kind in both"
    );
    assert_eq!(machine.counters().stored(), counters);
    assert_eq!(file_system.paths(), paths_before);
}

/// spec: IMPORT-018.6
#[test]
fn job_in_the_video_name_template_needs_a_job_code() {
    let file_system = photo_and_video();
    let paths_before = file_system.paths();

    let error = Machine::new()
        .try_import(
            &file_system,
            &Import::with(FLAT).with_video_template("{job}/{name}.{ext}"),
            &mut NoProgress,
        )
        .unwrap_err();

    assert!(
        matches!(
            error,
            RunError::MissingJobCode(MissingJobCode::VideoTemplate)
        ),
        "{error:?}"
    );
    assert!(error.to_string().contains("{job}"), "{error}");
    assert_eq!(file_system.paths(), paths_before);
}

/// spec: IMPORT-018.7
#[test]
fn a_video_name_template_too_narrow_keeps_the_run_from_starting() {
    let file_system = source_with(&[
        ("DSCF0001.RAF", &[1; 10]),
        ("DSCF0002.RAF", &[2; 10]),
        ("DSCF0003.RAF", &[3; 10]),
        ("DSCF0004.RAF", &[4; 10]),
        (VIDEO, &[5; 10]),
    ]);
    let paths_before = file_system.paths();
    let machine = Machine::new();
    machine.counters().set(StoredCounters {
        stored_last: 98,
        ..StoredCounters::default()
    });

    let error = machine
        .try_import(
            &file_system,
            &Import::with("{seq_stored:4}.{ext}").with_video_template("{seq_stored:2}.{ext}"),
            &mut NoProgress,
        )
        .unwrap_err();

    let RunError::Sequence(SequenceError::Overflow(overflow)) = &error else {
        panic!("overflow expected, got {error:?}");
    };
    assert_eq!(overflow.template, "{seq_stored:2}.{ext}");
    assert_eq!(overflow.suggestion.as_deref(), Some("{seq_stored:3}.{ext}"));
    assert_eq!(machine.stored_last(), 98);
    assert_eq!(file_system.paths(), paths_before);
}
