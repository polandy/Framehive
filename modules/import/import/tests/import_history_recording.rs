// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Recording verified copies and runs in the import history, backing up files imported
//! before, and a history that fails (IMPORT-009), against `MemoryFileSystem` and
//! `MemoryImportHistory`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers may panic on bad fixtures"
)]

mod history_support;

use std::path::{Path, PathBuf};

use domain::LocalDateTime;
use files::Hasher;
use files::testing::{Fault, FnvHasher};
use import::{BackupOutcome, ConflictPolicy, FileOutcome, FormatReadiness, NoProgress, RunError};

use history_support::{
    BACKUP, BY_DATE, DESTINATION, FLAT, Import, Machine, OTHER_DESTINATION, SOURCE, add, at,
    copied, imported_before, imported_once, paths_below, photo, recorded_at, report, source_with,
    stop_after,
};

/// spec: IMPORT-009.1
#[test]
fn a_verified_copy_is_recorded() {
    let (machine, _) = imported_once(BY_DATE);

    let [copy] = &machine.history.copies()[..] else {
        panic!("one copy expected, got {:?}", machine.history.copies());
    };
    assert_eq!(copy.destination, Path::new(DESTINATION));
    assert_eq!(copy.original_file_name, "DSCF0001.JPG");
    assert_eq!(copy.size, u64::try_from(photo(1).len()).unwrap());
    let captured = LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap();
    assert_eq!(copy.capture_time, Some(captured));
    let mut hash = FnvHasher.start();
    hash.update(&photo(1));
    assert_eq!(copy.checksum, hash.finish());
    assert_eq!(copy.target, "2026-10-17/DSCF0001.JPG");
}

/// spec: IMPORT-009.7
#[test]
fn another_import_destination_gets_the_file_too() {
    let (machine, file_system) = imported_once(BY_DATE);

    let summary = machine.import(&file_system, &Import::with(BY_DATE).into(OTHER_DESTINATION));

    assert_eq!(
        copied(report(&summary, "DSCF0001.JPG")),
        "2026-10-17/DSCF0001.JPG"
    );
    assert_eq!(
        recorded_at(&machine.history, "DSCF0001.JPG"),
        [PathBuf::from(DESTINATION), PathBuf::from(OTHER_DESTINATION)]
    );
}

/// spec: IMPORT-009.10
#[test]
fn a_re_run_completes_a_backup() {
    let machine = Machine::new();
    let failing = source_with(&[("DSCF0001.JPG", &photo(1))]);
    let target = "2026-10-17/DSCF0001.JPG";
    let part = at(BACKUP, &format!("{target}.part"));
    failing.inject_fault(&part, Fault::WriteFails { after_bytes: 0 });
    let first = machine.import(&failing, &Import::with(BY_DATE).backed_up());
    let backup = &report(&first, "DSCF0001.JPG").backups[0].outcome;
    assert!(matches!(backup, BackupOutcome::Failed(_)), "{backup:?}");
    let library = source_with(&[("DSCF0001.JPG", &photo(1))]);
    add(&library, DESTINATION, target, &photo(1));

    let summary = machine.import(&library, &Import::with(BY_DATE).backed_up());

    let file = report(&summary, "DSCF0001.JPG");
    assert_eq!(imported_before(file), target);
    assert!(matches!(file.backups[0].outcome, BackupOutcome::BackedUp));
    assert_eq!(library.content(&at(BACKUP, target)), Some(photo(1)));
    assert_eq!(
        recorded_at(&machine.history, "DSCF0001.JPG"),
        [PathBuf::from(DESTINATION), PathBuf::from(BACKUP)]
    );
}

/// spec: IMPORT-009.11
#[test]
fn backed_up_before_writes_nothing() {
    let machine = Machine::new();
    let file_system = source_with(&[("DSCF0001.JPG", &photo(1))]);
    machine.import(&file_system, &Import::with(BY_DATE).backed_up());
    let emptied = source_with(&[("DSCF0001.JPG", &photo(1))]);
    let paths = emptied.paths();

    let summary = machine.import(&emptied, &Import::with(BY_DATE).backed_up());

    let file = report(&summary, "DSCF0001.JPG");
    assert_eq!(imported_before(file), "2026-10-17/DSCF0001.JPG");
    assert!(matches!(
        file.backups[0].outcome,
        BackupOutcome::BackedUpBefore
    ));
    assert_eq!(emptied.paths(), paths);
}

/// spec: IMPORT-009.12
#[test]
fn a_file_already_imported_is_recorded() {
    let machine = Machine::new();
    let file_system = source_with(&[("DSCF0001.JPG", &photo(1))]);
    add(
        &file_system,
        DESTINATION,
        "2026-10-17/DSCF0001.JPG",
        &photo(1),
    );

    let summary = machine.import(&file_system, &Import::with(BY_DATE));

    assert!(matches!(
        &report(&summary, "DSCF0001.JPG").outcome,
        FileOutcome::AlreadyImported { target } if target == "2026-10-17/DSCF0001.JPG"
    ));
    assert_eq!(
        recorded_at(&machine.history, "DSCF0001.JPG"),
        [PathBuf::from(DESTINATION)]
    );
    let renamed = source_with(&[("DSCF0001.JPG", &photo(1))]);
    add(&renamed, DESTINATION, "keeper.JPG", &photo(1));
    let again = machine.import(&renamed, &Import::with(BY_DATE));
    let target = imported_before(report(&again, "DSCF0001.JPG"));
    assert_eq!(target, "2026-10-17/DSCF0001.JPG");
}

/// spec: IMPORT-009.13
#[test]
fn files_that_are_not_verified_are_not_recorded() {
    let machine = Machine::new();
    let file_system = source_with(&[
        ("a.JPG", &photo(1)),
        ("more/a.JPG", &photo(1)),
        ("b.JPG", &photo(2)),
        ("c.JPG", &photo(3)),
        ("d.JPG", &photo(4)),
    ]);
    add(&file_system, DESTINATION, "b.JPG", b"another file");
    let fault = Fault::WriteFails { after_bytes: 0 };
    file_system.inject_fault(&at(DESTINATION, "c.JPG.part"), fault.clone());
    file_system.inject_fault(&at(BACKUP, "d.JPG.part"), fault);
    let import = Import::with(FLAT)
        .backed_up()
        .with_policy(ConflictPolicy::Skip);

    let summary = machine.import(&file_system, &import);

    let outcome = |path| &report(&summary, path).outcome;
    assert!(matches!(
        outcome("more/a.JPG"),
        FileOutcome::DuplicateOf { .. }
    ));
    assert!(matches!(
        outcome("b.JPG"),
        FileOutcome::SkippedNameConflict { .. }
    ));
    assert!(matches!(outcome("c.JPG"), FileOutcome::Failed(_)));
    for not_verified in ["more/a.JPG", "b.JPG", "c.JPG"] {
        assert_eq!(
            recorded_at(&machine.history, not_verified),
            [] as [PathBuf; 0]
        );
    }
    assert_eq!(
        recorded_at(&machine.history, "d.JPG"),
        [PathBuf::from(DESTINATION)]
    );
    assert_eq!(
        recorded_at(&machine.history, "a.JPG"),
        [PathBuf::from(DESTINATION), PathBuf::from(BACKUP)]
    );
}

/// spec: IMPORT-009.15
#[test]
fn a_history_that_cannot_be_used_does_not_start_the_run() {
    let machine = Machine::new();
    let file_system = source_with(&[("DSCF0001.JPG", &photo(1))]);
    machine.history.break_storage();

    let error = machine
        .try_import(&file_system, &Import::with(BY_DATE), &mut NoProgress)
        .unwrap_err();

    assert!(matches!(error, RunError::History(_)), "{error:?}");
    assert!(error.to_string().contains("cannot be used"), "{error}");
    assert_eq!(paths_below(&file_system, DESTINATION), [] as [PathBuf; 0]);
}

/// spec: IMPORT-009.16
#[test]
fn a_history_that_cannot_record_a_copy_stops_the_run_after_that_file() {
    let machine = Machine::new();
    let file_system = source_with(&[
        ("a.JPG", &photo(1)),
        ("b.JPG", &photo(2)),
        ("c.JPG", &photo(3)),
    ]);
    machine.history.fail_recording_after(1);

    let summary = machine.import(&file_system, &Import::with(FLAT));

    assert_eq!(copied(report(&summary, "a.JPG")), "a.JPG");
    assert_eq!(copied(report(&summary, "b.JPG")), "b.JPG");
    assert_eq!(summary.files.len(), 2);
    assert!(summary.history_error.is_some());
    assert!(summary.has_failures());
    assert_eq!(file_system.content(&at(DESTINATION, "c.JPG")), None);
    assert!(machine.history.runs()[0].finished_at.is_some());
}

/// spec: IMPORT-009.17
#[test]
fn format_readiness_counts_recorded_copies() {
    let machine = Machine::new();
    let file_system = source_with(&[("DSCF0001.JPG", &photo(1))]);
    machine.import(&file_system, &Import::with(BY_DATE).backed_up());
    let emptied = source_with(&[("DSCF0001.JPG", &photo(1))]);

    let summary = machine.import(&emptied, &Import::with(BY_DATE).backed_up());

    let readiness = FormatReadiness::of(&summary);
    assert!(readiness.can_be_formatted(), "{readiness:?}");
    assert_eq!(readiness.required_locations, 2);
}

/// spec: IMPORT-009.18
#[test]
fn a_stopped_run_records_its_start_and_end() {
    let machine = Machine::new();
    let file_system = source_with(&[("a.JPG", &photo(1)), ("b.JPG", &photo(2))]);

    machine
        .try_import(&file_system, &Import::with(FLAT), &mut stop_after(1))
        .unwrap();

    let [run] = &machine.history.runs()[..] else {
        panic!("one run expected, got {:?}", machine.history.runs());
    };
    assert_eq!(run.start.source, Path::new(SOURCE));
    assert_eq!(run.start.import_destination, Path::new(DESTINATION));
    assert!(run.finished_at.is_some());
}
