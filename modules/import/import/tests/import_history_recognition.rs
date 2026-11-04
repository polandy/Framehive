// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Recognizing files imported before by the import history (IMPORT-009), against
//! `MemoryFileSystem` and `MemoryImportHistory`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers may panic on bad fixtures"
)]

mod history_support;

use std::path::Path;

use domain::Timestamp;
use files::testing::MemoryFileSystem;
use import::{FileOutcome, SessionSequence, scan_source};

use history_support::{
    BY_DATE, DESTINATION, FLAT, Import, Machine, NOW, SOURCE, STORED, add, at, copied,
    imported_before, imported_once, photo, recorded_at, report, source_with, stop_after,
};

/// spec: IMPORT-009.2
#[test]
fn a_persisted_counter_does_not_copy_a_file_again() {
    let (machine, file_system) = imported_once(STORED);
    let paths = file_system.paths();

    let summary = machine.import(&file_system, &Import::with(STORED));

    assert_eq!(
        imported_before(report(&summary, "DSCF0001.JPG")),
        "0001.JPG"
    );
    assert_eq!(file_system.paths(), paths);
    assert_eq!(machine.stored_last(), 1);
}

/// spec: IMPORT-009.3
#[test]
fn a_file_renamed_or_deleted_in_the_library_is_still_recognized() {
    let (machine, _) = imported_once(BY_DATE);
    let renamed = source_with(&[("DSCF0001.JPG", &photo(1))]);
    add(&renamed, DESTINATION, "best/keeper.JPG", &photo(1));
    let deleted = source_with(&[("DSCF0001.JPG", &photo(1))]);

    for library in [renamed, deleted] {
        let paths = library.paths();
        let summary = machine.import(&library, &Import::with(BY_DATE));

        let target = imported_before(report(&summary, "DSCF0001.JPG"));
        assert_eq!(target, "2026-10-17/DSCF0001.JPG");
        assert_eq!(library.paths(), paths);
    }
}

/// spec: IMPORT-009.4
#[test]
fn same_name_and_size_with_other_content_is_not_imported_before() {
    let (machine, _) = imported_once(BY_DATE);
    let other = source_with(&[("DSCF0001.JPG", &photo(2))]);

    let summary = machine.import(&other, &Import::with(BY_DATE));

    let target = copied(report(&summary, "DSCF0001.JPG"));
    assert_eq!(target, "2026-10-17/DSCF0001.JPG");
    assert_eq!(recorded_at(&machine.history, "DSCF0001.JPG").len(), 2);
}

/// spec: IMPORT-009.5
#[test]
fn a_card_without_matches_is_not_read_before_the_run() {
    let (machine, _) = imported_once(BY_DATE);
    let longer = [photo(1), b"more".to_vec()].concat();
    let other = source_with(&[("DSCF0001.JPG", &longer), ("DSCF0002.JPG", &photo(1))]);
    let import = Import::with(BY_DATE);
    let settings = import.settings();
    let scan = scan_source(&other, Path::new(SOURCE), Path::new(DESTINATION)).unwrap();
    let session = SessionSequence::new();
    let hashes_before = machine.hasher.started.get();

    let plan = machine
        .run(&other, &session)
        .plan(&settings, &scan)
        .unwrap();

    assert_eq!(plan.files().len(), 2);
    assert_eq!(machine.hasher.started.get(), hashes_before);
}

/// spec: IMPORT-009.6
#[test]
fn a_file_without_a_capture_time_is_matched_by_name_size_and_checksum() {
    let (machine, file_system) = (Machine::new(), source_with(&[("notes.png", b"notes")]));
    machine.import(&file_system, &Import::with(FLAT));
    assert_eq!(machine.history.copies()[0].capture_time, None);
    let touched = MemoryFileSystem::new();
    for folder in [SOURCE, DESTINATION] {
        touched.add_folder(Path::new(folder));
    }
    touched.add_file(
        &at(SOURCE, "notes.png"),
        b"notes",
        Timestamp::from_unix_seconds(NOW),
    );

    let summary = machine.import(&touched, &Import::with(FLAT));

    assert_eq!(imported_before(report(&summary, "notes.png")), "notes.png");
}

/// spec: IMPORT-009.8
#[test]
fn a_changed_template_does_not_make_a_file_new() {
    let (machine, file_system) = imported_once(BY_DATE);

    let summary = machine.import(&file_system, &Import::with(FLAT));

    let target = imported_before(report(&summary, "DSCF0001.JPG"));
    assert_eq!(target, "2026-10-17/DSCF0001.JPG");
    assert_eq!(file_system.content(&at(DESTINATION, "DSCF0001.JPG")), None);
}

/// spec: IMPORT-009.9
#[test]
fn resuming_an_interrupted_import_copies_only_the_rest() {
    let machine = Machine::new();
    let file_system = source_with(&[
        ("a.JPG", &photo(1)),
        ("b.JPG", &photo(2)),
        ("c.JPG", &photo(3)),
    ]);
    let import = Import::with(STORED);
    machine
        .try_import(&file_system, &import, &mut stop_after(1))
        .unwrap();

    let summary = machine.import(&file_system, &import);

    assert_eq!(imported_before(report(&summary, "a.JPG")), "0001.JPG");
    assert_eq!(copied(report(&summary, "b.JPG")), "0004.JPG");
    assert_eq!(copied(report(&summary, "c.JPG")), "0005.JPG");
    for unused in ["0002.JPG", "0003.JPG"] {
        assert_eq!(file_system.content(&at(DESTINATION, unused)), None);
    }
    assert_eq!(machine.stored_last(), 5);
}

/// spec: IMPORT-009.14
#[test]
fn a_dry_run_reads_the_history_and_records_nothing() {
    let (machine, file_system) = imported_once(BY_DATE);
    add(&file_system, SOURCE, "DSCF0002.JPG", &photo(2));
    let (runs, copies) = (machine.history.runs(), machine.history.copies());

    let summary = machine.import(&file_system, &Import::with(BY_DATE).dry_running());

    assert_eq!(
        imported_before(report(&summary, "DSCF0001.JPG")),
        "2026-10-17/DSCF0001.JPG"
    );
    assert!(matches!(
        &report(&summary, "DSCF0002.JPG").outcome,
        FileOutcome::WouldCopy { .. }
    ));
    assert_eq!(machine.history.runs(), runs);
    assert_eq!(machine.history.copies(), copies);
}
