// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The free-space check before the first copy (IMPORT-012), against
//! `MemoryFileSystem` and `MemoryFreeSpace`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers may panic on bad fixtures"
)]

mod backup_support;
mod history_support;

use std::path::{Path, PathBuf};

use files::testing::{Fault, MemoryFileSystem, MemoryFreeSpace};
use import::{BackupOutcome, FileOutcome, ImportSummary, RunError, SpaceShortage};

use backup_support::{BACKUP, DESTINATION, FLAT, SOURCE, Setup, at, modified, paths_below};

/// The files of IMPORT-012.1: 400 and 600 bytes, 1000 bytes together.
const FILES: [(&str, usize); 2] = [("DSCF0001.RAF", 400), ("DSCF0002.RAF", 600)];

/// A file system with the files of IMPORT-012.1 in the source, the backup
/// destination, and the import destination if `destination_exists`.
fn two_files(destination_exists: bool) -> MemoryFileSystem {
    let file_system = MemoryFileSystem::new();
    file_system.add_folder(Path::new(SOURCE));
    file_system.add_folder(Path::new(BACKUP));
    if destination_exists {
        file_system.add_folder(Path::new(DESTINATION));
    }
    for (index, (name, size)) in FILES.iter().enumerate() {
        let content = vec![u8::try_from(index).unwrap(); *size];
        file_system.add_file(&at(SOURCE, name), &content, modified());
    }
    file_system
}

/// One file system for the whole machine, with `available_bytes` free.
fn one_file_system(available_bytes: u64) -> MemoryFreeSpace {
    MemoryFreeSpace::new().with_file_system("/", available_bytes)
}

/// The import destination and the backup destination on two file systems.
fn two_file_systems(at_destination: u64, at_backup: u64) -> MemoryFreeSpace {
    MemoryFreeSpace::new()
        .with_file_system(DESTINATION, at_destination)
        .with_file_system(BACKUP, at_backup)
}

fn copied(summary: &ImportSummary) -> Vec<&str> {
    summary
        .files
        .iter()
        .filter_map(|report| match &report.outcome {
            FileOutcome::Copied { target, .. } => Some(target.as_str()),
            _ => None,
        })
        .collect()
}

fn shortages(result: Result<impl std::fmt::Debug, RunError>) -> Vec<SpaceShortage> {
    match result {
        Err(RunError::NotEnoughSpace(error)) => error.shortages,
        other => panic!("not enough space expected, got {other:?}"),
    }
}

fn shortage(destinations: &[&str], bytes_to_write: u64, available_bytes: u64) -> SpaceShortage {
    SpaceShortage {
        destinations: destinations.iter().map(PathBuf::from).collect(),
        bytes_to_write,
        available_bytes,
    }
}

/// spec: IMPORT-012.1
#[test]
fn an_import_that_fits_starts() {
    let file_system = two_files(true);

    let summary = Setup::new(FLAT, &[])
        .with_free_space(one_file_system(1000))
        .on(&file_system);

    assert_eq!(copied(&summary), ["DSCF0001.RAF", "DSCF0002.RAF"]);
    assert!(summary.free_space_unknown.is_empty());
}

/// spec: IMPORT-012.2
#[test]
fn an_import_that_does_not_fit_does_not_start() {
    let file_system = two_files(false);
    let sources = FILES.map(|(name, _)| at(SOURCE, name));

    let result = Setup::new(FLAT, &[])
        .with_free_space(one_file_system(999))
        .try_start_on(&file_system, || {
            // Planning has read the capture times; from here on, a read fails.
            for source in &sources {
                file_system.inject_fault(source, Fault::ReadFails { after_bytes: 0 });
            }
        });

    assert_eq!(shortages(result), [shortage(&[DESTINATION], 1000, 999)]);
    assert!(!file_system.paths().contains(&PathBuf::from(DESTINATION)));
}

/// spec: IMPORT-012.3
#[test]
fn destinations_on_one_file_system_add_up() {
    let file_system = two_files(true);

    let result = Setup::new(FLAT, &[BACKUP])
        .with_free_space(one_file_system(1500))
        .try_on(&file_system);

    assert_eq!(
        shortages(result),
        [shortage(&[DESTINATION, BACKUP], 2000, 1500)]
    );
    assert_eq!(
        paths_below(&file_system, DESTINATION),
        Vec::<PathBuf>::new()
    );
    assert_eq!(paths_below(&file_system, BACKUP), Vec::<PathBuf>::new());
}

/// spec: IMPORT-012.4
#[test]
fn each_file_system_is_checked_on_its_own() {
    let file_system = two_files(true);

    let summary = Setup::new(FLAT, &[BACKUP])
        .with_free_space(two_file_systems(1000, 1000))
        .on(&file_system);

    assert_eq!(copied(&summary), ["DSCF0001.RAF", "DSCF0002.RAF"]);
    for report in &summary.files {
        assert!(matches!(report.backups[0].outcome, BackupOutcome::BackedUp));
    }
}

/// spec: IMPORT-012.5
#[test]
fn files_not_written_do_not_count() {
    use history_support::{FLAT, Import, Machine, SOURCE, add, source_with};

    let content = |variant: u8| vec![variant; 500];
    let mut machine = Machine::new();
    let file_system = source_with(&[("DSCF0001.JPG", &content(1))]);
    machine.import(&file_system, &Import::with(FLAT));
    add(&file_system, SOURCE, "DSCF0002.JPG", &content(2));
    add(
        &file_system,
        history_support::DESTINATION,
        "DSCF0002.JPG",
        &content(2),
    );
    add(&file_system, SOURCE, "DSCF0003.JPG", &content(3));
    machine.free_space = one_file_system(500);

    let summary = machine.import(&file_system, &Import::with(FLAT));

    let outcomes: Vec<&FileOutcome> = summary.files.iter().map(|file| &file.outcome).collect();
    assert!(matches!(outcomes[0], FileOutcome::ImportedBefore { .. }));
    assert!(matches!(outcomes[1], FileOutcome::AlreadyImported { .. }));
    assert_eq!(copied(&summary), ["DSCF0003.JPG"]);
}

/// spec: IMPORT-012.6
#[test]
fn files_already_at_a_backup_target_do_not_count_there() {
    let file_system = two_files(true);
    let first = file_system.content(&at(SOURCE, FILES[0].0)).unwrap();
    file_system.add_file(&at(BACKUP, FILES[0].0), &first, modified());

    let summary = Setup::new(FLAT, &[BACKUP])
        .with_free_space(two_file_systems(1000, 600))
        .on(&file_system);

    assert_eq!(copied(&summary), ["DSCF0001.RAF", "DSCF0002.RAF"]);
    let backups: Vec<&BackupOutcome> = summary
        .files
        .iter()
        .map(|file| &file.backups[0].outcome)
        .collect();
    assert!(matches!(backups[0], BackupOutcome::AlreadyBackedUp));
    assert!(matches!(backups[1], BackupOutcome::BackedUp));
}

/// spec: IMPORT-012.7
#[test]
fn every_short_file_system_is_reported() {
    let file_system = two_files(true);

    let result = Setup::new(FLAT, &[BACKUP])
        .with_free_space(two_file_systems(999, 999))
        .try_on(&file_system);

    assert_eq!(
        shortages(result),
        [
            shortage(&[DESTINATION], 1000, 999),
            shortage(&[BACKUP], 1000, 999)
        ]
    );
}

/// spec: IMPORT-012.8
#[test]
fn unknown_free_space_warns_and_goes_on() {
    let file_system = two_files(true);
    let free_space = MemoryFreeSpace::new().with_unreadable("/", "not supported here");

    let summary = Setup::new(FLAT, &[])
        .with_free_space(free_space)
        .on(&file_system);

    assert_eq!(copied(&summary), ["DSCF0001.RAF", "DSCF0002.RAF"]);
    let [unknown] = &summary.free_space_unknown[..] else {
        panic!("one warning expected, got {:?}", summary.free_space_unknown);
    };
    assert_eq!(unknown.destination, Path::new(DESTINATION));
    assert_eq!(unknown.error.to_string(), "not supported here");
}

/// spec: IMPORT-012.9
#[test]
fn a_dry_run_checks_too() {
    let file_system = two_files(false);

    let result = Setup::new(FLAT, &[])
        .with_free_space(one_file_system(999))
        .dry_running()
        .try_on(&file_system);

    assert_eq!(shortages(result), [shortage(&[DESTINATION], 1000, 999)]);
    assert!(!file_system.paths().contains(&PathBuf::from(DESTINATION)));
}
