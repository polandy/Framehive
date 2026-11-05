// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sequence numbers of an import run: previewed when planned, reserved when it starts
//! (IMPORT-002, "Reservation"), against `MemoryFileSystem` and
//! `MemorySequenceCounters`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::Path;
use std::sync::LazyLock;

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{FnvHasher, MemoryFileSystem, MemoryFreeSpace};
use history::testing::MemoryImportHistory;
use import::{
    ConflictPolicy, FileOutcome, ImportRun, ImportSettings, ImportSummary, NoProgress, PlannedRun,
    RunError, SessionSequence, SourceScan, scan_source,
};
use naming::testing::MemorySequenceCounters;
use naming::{
    CompatibilityMode, CounterDate, NameTemplate, SequenceCountersError, SequenceError,
    StoredCounters,
};

const SOURCE: &str = "/media/card";
const DESTINATION: &str = "/library";
/// 2026-10-17 14:05:09 UTC.
const MODIFIED: i64 = 1_792_245_909;
/// 2026-10-31 09:00:00 UTC.
const NOW: i64 = 1_793_437_200;

/// An import run through the memory ports, on a file system with space for any import.
fn import_run<'a>(
    file_system: &'a MemoryFileSystem,
    clock: &'a ManualClock,
    counters: &'a MemorySequenceCounters,
    session: &'a SessionSequence,
    history: &'a MemoryImportHistory,
) -> ImportRun<'a> {
    static UNLIMITED: LazyLock<MemoryFreeSpace> = LazyLock::new(MemoryFreeSpace::unlimited);
    ImportRun::new(
        file_system,
        &FnvHasher,
        clock,
        counters,
        session,
        history,
        &*UNLIMITED,
    )
}

/// Videos are dated by their modification time, so all three share one time and are
/// numbered by base name.
fn card() -> MemoryFileSystem {
    let file_system = MemoryFileSystem::new();
    file_system.add_folder(Path::new(SOURCE));
    for file in ["a.MOV", "b.MOV", "c.MOV"] {
        file_system.add_file(
            &Path::new(SOURCE).join(file),
            file.as_bytes(),
            Timestamp::from_unix_seconds(MODIFIED),
        );
    }
    file_system
}

fn clock() -> ManualClock {
    ManualClock::new(Timestamp::from_unix_seconds(NOW), UtcOffset::UTC)
}

fn stored_at(last: u32) -> MemorySequenceCounters {
    let counters = MemorySequenceCounters::new();
    counters.set(StoredCounters {
        stored_last: last,
        ..StoredCounters::default()
    });
    counters
}

fn settings(template: &NameTemplate, dry_run: bool) -> ImportSettings<'_> {
    ImportSettings {
        source: Path::new(SOURCE),
        destination: Path::new(DESTINATION),
        backup_destinations: &[],
        template,
        job_code: None,
        mode: CompatibilityMode::On,
        policy: ConflictPolicy::AddSuffix,
        dry_run,
    }
}

fn scan(file_system: &MemoryFileSystem) -> SourceScan {
    scan_source(file_system, Path::new(SOURCE), Path::new(DESTINATION)).unwrap()
}

fn planned(plan: &PlannedRun) -> Vec<&str> {
    plan.files()
        .iter()
        .map(|file| {
            file.planned
                .as_ref()
                .unwrap()
                .named()
                .unwrap()
                .target
                .as_deref()
                .unwrap()
        })
        .collect()
}

fn copied(summary: &ImportSummary) -> Vec<&str> {
    summary
        .files
        .iter()
        .filter_map(|report| match &report.outcome {
            FileOutcome::Copied { target, .. } | FileOutcome::WouldCopy { target } => {
                Some(target.as_str())
            }
            _ => None,
        })
        .collect()
}

/// spec: IMPORT-002.9
#[test]
fn a_dry_run_shows_the_numbers_and_reserves_nothing() {
    let file_system = card();
    let (clock, counters, session) = (clock(), stored_at(250), SessionSequence::new());
    let history = MemoryImportHistory::new();
    let run = import_run(&file_system, &clock, &counters, &session, &history);
    let template = NameTemplate::parse("{seq_stored:4}.{ext}").unwrap();

    let dry_run = run
        .run(&settings(&template, true), &mut NoProgress)
        .unwrap();
    let stored_after_dry_run = counters.stored().stored_last;
    let imported = run
        .run(&settings(&template, false), &mut NoProgress)
        .unwrap();

    assert_eq!(copied(&dry_run), ["0251.MOV", "0252.MOV", "0253.MOV"]);
    assert_eq!(stored_after_dry_run, 250);
    assert_eq!(copied(&imported), ["0251.MOV", "0252.MOV", "0253.MOV"]);
    assert_eq!(counters.stored().stored_last, 253);
}

/// spec: IMPORT-002.4
#[test]
fn an_import_that_started_in_between_moves_the_numbers_on() {
    let file_system = card();
    let (clock, counters, session) = (clock(), stored_at(100), SessionSequence::new());
    let history = MemoryImportHistory::new();
    let run = import_run(&file_system, &clock, &counters, &session, &history);
    let template = NameTemplate::parse("{seq_stored:4}.{ext}").unwrap();
    let settings = settings(&template, false);

    let plan = run.plan(&settings, &scan(&file_system)).unwrap();
    let shown = planned(&plan).join(" ");
    // Another process reserves 101-110 while the summary is shown.
    counters.set(StoredCounters {
        stored_last: 110,
        ..StoredCounters::default()
    });
    let started = run.start(&settings, plan).unwrap();

    assert_eq!(shown, "0101.MOV 0102.MOV 0103.MOV");
    assert_eq!(planned(&started), ["0111.MOV", "0112.MOV", "0113.MOV"]);
    assert_eq!(counters.stored().stored_last, 113);
}

/// spec: IMPORT-002.1
#[test]
fn the_imported_today_counter_belongs_to_the_day_the_import_starts() {
    let file_system = card();
    let (clock, session) = (clock(), SessionSequence::new());
    let counters = MemorySequenceCounters::new();
    let today = CounterDate::parse("2026-10-31").unwrap();
    counters.set(StoredCounters {
        imported_today_date: Some(today),
        imported_today_last: 12,
        stored_last: 0,
    });
    let history = MemoryImportHistory::new();
    let run = import_run(&file_system, &clock, &counters, &session, &history);
    let template = NameTemplate::parse("{date:%Y%m%d}_{seq_today:3}.{ext}").unwrap();

    let summary = run
        .run(&settings(&template, false), &mut NoProgress)
        .unwrap();

    assert_eq!(
        copied(&summary),
        ["20261017_013.MOV", "20261017_014.MOV", "20261017_015.MOV"]
    );
    assert_eq!(counters.stored().imported_today_last, 15);
}

/// spec: IMPORT-002.7
#[test]
fn an_overflow_keeps_the_import_from_starting() {
    let file_system = card();
    let paths_before = file_system.paths();
    let (clock, counters, session) = (clock(), stored_at(98), SessionSequence::new());
    let history = MemoryImportHistory::new();
    let run = import_run(&file_system, &clock, &counters, &session, &history);
    let template = NameTemplate::parse("{seq_stored:2}.{ext}").unwrap();

    let error = run
        .run(&settings(&template, false), &mut NoProgress)
        .unwrap_err();

    assert!(
        matches!(error, RunError::Sequence(SequenceError::Overflow(_))),
        "{error:?}"
    );
    assert_eq!(counters.stored().stored_last, 98);
    assert_eq!(file_system.paths(), paths_before);
}

/// spec: IMPORT-002.10
#[test]
fn broken_counters_keep_the_import_from_starting() {
    let file_system = card();
    let paths_before = file_system.paths();
    let (clock, counters, session) = (clock(), stored_at(250), SessionSequence::new());
    counters.break_at("/state/framehive/sequence-counters.txt", 3);
    let history = MemoryImportHistory::new();
    let run = import_run(&file_system, &clock, &counters, &session, &history);
    let template = NameTemplate::parse("{seq_stored:4}.{ext}").unwrap();

    let error = run
        .run(&settings(&template, false), &mut NoProgress)
        .unwrap_err();

    assert!(
        matches!(
            error,
            RunError::Sequence(SequenceError::Counters(SequenceCountersError::Broken {
                line: 3,
                ..
            }))
        ),
        "{error:?}"
    );
    assert_eq!(file_system.paths(), paths_before);
}

/// spec: IMPORT-002.11
#[test]
fn a_session_counter_runs_with_broken_counters() {
    let file_system = card();
    let (clock, counters, session) = (clock(), stored_at(250), SessionSequence::new());
    counters.break_at("/state/framehive/sequence-counters.txt", 3);
    let history = MemoryImportHistory::new();
    let run = import_run(&file_system, &clock, &counters, &session, &history);
    let template = NameTemplate::parse("{seq_session:4}.{ext}").unwrap();

    let summary = run
        .run(&settings(&template, false), &mut NoProgress)
        .unwrap();

    assert_eq!(copied(&summary), ["0001.MOV", "0002.MOV", "0003.MOV"]);
}

#[test]
fn imports_of_one_session_continue_its_numbers() {
    let (first_card, second_card) = (card(), card());
    let (clock, counters, session) = (
        clock(),
        MemorySequenceCounters::new(),
        SessionSequence::new(),
    );
    let template = NameTemplate::parse("{seq_letters}.{ext}").unwrap();

    let first = import_run(
        &first_card,
        &clock,
        &counters,
        &session,
        &MemoryImportHistory::new(),
    )
    .run(&settings(&template, false), &mut NoProgress)
    .unwrap();
    let second = import_run(
        &second_card,
        &clock,
        &counters,
        &session,
        &MemoryImportHistory::new(),
    )
    .run(&settings(&template, false), &mut NoProgress)
    .unwrap();

    assert_eq!(copied(&first), ["a.MOV", "b.MOV", "c.MOV"]);
    assert_eq!(copied(&second), ["d.MOV", "e.MOV", "f.MOV"]);
    assert_eq!(session.last(), 6);
}

/// spec: IMPORT-002.14
#[test]
fn a_session_counter_recognizes_files_imported_in_an_earlier_run() {
    let file_system = card();
    let (clock, counters) = (clock(), MemorySequenceCounters::new());
    let template = NameTemplate::parse("{date:%Y%m%d}_{seq_session:3}.{ext}").unwrap();
    let import_in_new_run = || {
        let session = SessionSequence::new();
        import_run(
            &file_system,
            &clock,
            &counters,
            &session,
            &MemoryImportHistory::new(),
        )
        .run(&settings(&template, false), &mut NoProgress)
        .unwrap()
    };

    let first = import_in_new_run();
    let paths_after_first = file_system.paths();
    let again = import_in_new_run();

    assert_eq!(copied(&first).len(), 3);
    assert!(
        again
            .files
            .iter()
            .all(|report| matches!(report.outcome, FileOutcome::AlreadyImported { .. })),
        "{again:?}"
    );
    assert_eq!(file_system.paths(), paths_after_first);
}

/// spec: IMPORT-002.15
#[test]
fn a_persisted_counter_recognizes_files_imported_before() {
    let file_system = card();
    let (clock, counters, session) = (
        clock(),
        MemorySequenceCounters::new(),
        SessionSequence::new(),
    );
    let history = MemoryImportHistory::new();
    let run = import_run(&file_system, &clock, &counters, &session, &history);
    let template = NameTemplate::parse("{seq_today:3}.{ext}").unwrap();

    let first = run
        .run(&settings(&template, false), &mut NoProgress)
        .unwrap();
    let (counters_after_first, paths_after_first) = (counters.stored(), file_system.paths());
    let again = run
        .run(&settings(&template, false), &mut NoProgress)
        .unwrap();

    assert_eq!(copied(&first), ["001.MOV", "002.MOV", "003.MOV"]);
    assert!(
        again
            .files
            .iter()
            .all(|report| matches!(report.outcome, FileOutcome::ImportedBefore { .. })),
        "{again:?}"
    );
    assert_eq!(counters.stored(), counters_after_first);
    assert_eq!(file_system.paths(), paths_after_first);
}
