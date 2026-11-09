// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The sidecars at the backup destinations: a leftover one takes the file's suffix as
//! at the import destination, and a re-run completes one that failed only at a backup
//! (IMPORT-007, IMPORT-019).

#![allow(clippy::unwrap_used, reason = "tests may panic")]

mod sidecar_support;

use std::path::Path;
use std::time::Duration;

use files::Hasher;
use files::testing::{Fault, FnvHasher, MemoryFileSystem};
use import::{
    BackupError, BackupOutcome, ConflictPolicy, FileOutcome, FileReport, FormatReadiness,
};
use sidecar_support::{
    BACKUPS, DESTINATION, FLAT, Import, Machine, SOURCE, card_with, copied, put, raw_taken_at,
    report, sidecars_below,
};

const RAW: &str = "DSCF0042.RAF";
const SIDECAR: &str = "/library/DSCF0042.RAF.xmp";
const AT_BACKUP: &str = "/backup-1/DSCF0042.RAF.xmp";

fn raw_card() -> MemoryFileSystem {
    card_with(&[(RAW, &raw_taken_at("2026:10:17 14:05:09"))])
}

fn checksum(content: &[u8]) -> domain::Checksum {
    let mut hash = FnvHasher.start();
    hash.update(content);
    hash.finish()
}

fn content(card: &MemoryFileSystem, path: &str) -> Option<Vec<u8>> {
    card.content(Path::new(path))
}

/// Asserts that the sidecar `name` has the bytes of the import destination's at each
/// backup destination.
fn assert_same_at_backups(card: &MemoryFileSystem, name: &str) {
    let written = content(card, &format!("{DESTINATION}/{name}")).unwrap();
    for backup in BACKUPS {
        let path = format!("{backup}/{name}");
        assert_eq!(content(card, &path).as_ref(), Some(&written), "{path}");
    }
}

fn backup_outcome<'r>(file: &'r FileReport, destination: &str) -> &'r BackupOutcome {
    &file
        .backups
        .iter()
        .find(|backup| backup.destination == Path::new(destination))
        .unwrap()
        .outcome
}

/// spec: IMPORT-007.14
#[test]
fn a_leftover_sidecar_at_a_backup_takes_the_suffix() {
    let machine = Machine::new();
    let card = raw_card();
    put(&card, Path::new(AT_BACKUP), b"leftover");
    let summary = machine.import(&card, &Import::lisbon(FLAT).backed_up());
    let file = report(&summary, RAW);
    assert_eq!(copied(file), "DSCF0042-1.RAF");
    assert_eq!(file.sidecar.as_deref(), Some("DSCF0042-1.RAF.xmp"));
    assert_eq!(content(&card, AT_BACKUP), Some(b"leftover".to_vec()));
    assert_same_at_backups(&card, "DSCF0042-1.RAF");
    assert_same_at_backups(&card, "DSCF0042-1.RAF.xmp");
    for counts in &summary.backups {
        assert_eq!((counts.backed_up, counts.failed), (1, 0), "{counts:?}");
    }
    assert!(FormatReadiness::of(&summary).can_be_formatted());
}

/// spec: IMPORT-007.14
#[test]
fn a_second_conflict_at_another_backup_takes_the_next_suffix() {
    let machine = Machine::new();
    let card = raw_card();
    put(&card, Path::new(AT_BACKUP), b"leftover");
    put(
        &card,
        Path::new("/backup-2/DSCF0042-1.RAF.xmp"),
        b"another leftover",
    );
    let summary = machine.import(&card, &Import::lisbon(FLAT).backed_up());
    assert_eq!(copied(report(&summary, RAW)), "DSCF0042-2.RAF");
    assert_same_at_backups(&card, "DSCF0042-2.RAF.xmp");
    assert_eq!(
        content(&card, "/backup-2/DSCF0042-1.RAF.xmp"),
        Some(b"another leftover".to_vec())
    );
}

/// spec: IMPORT-007.14
#[test]
fn a_missing_backup_destination_is_created_with_the_suffixed_names() {
    let machine = Machine::new();
    let card = MemoryFileSystem::new();
    for folder in [SOURCE, DESTINATION, BACKUPS[0]] {
        card.add_folder(Path::new(folder));
    }
    put(
        &card,
        &Path::new(SOURCE).join(RAW),
        &raw_taken_at("2026:10:17 14:05:09"),
    );
    put(&card, Path::new(AT_BACKUP), b"leftover");
    let summary = machine.import(&card, &Import::lisbon(FLAT).backed_up());
    assert_eq!(copied(report(&summary, RAW)), "DSCF0042-1.RAF");
    assert_same_at_backups(&card, "DSCF0042-1.RAF.xmp");
}

/// spec: IMPORT-007.14
#[test]
fn a_leftover_sidecar_at_a_backup_skips_the_file_under_skip() {
    let machine = Machine::new();
    let card = raw_card();
    put(&card, Path::new(AT_BACKUP), b"leftover");
    let mut import = Import::lisbon(FLAT).backed_up();
    import.policy = ConflictPolicy::Skip;
    let summary = machine.import(&card, &import);
    let file = report(&summary, RAW);
    assert!(
        matches!(file.outcome, FileOutcome::SkippedNameConflict { .. }),
        "{:?}",
        file.outcome
    );
    assert_eq!(
        sidecars_below(&card, DESTINATION),
        Vec::<std::path::PathBuf>::new()
    );
    assert_eq!(content(&card, "/backup-2/DSCF0042.RAF"), None);
}

/// spec: IMPORT-007.15
#[test]
fn a_re_run_completes_a_sidecar_that_failed_only_at_a_backup() {
    let machine = Machine::new();
    let card = raw_card();
    card.inject_fault(Path::new(AT_BACKUP), Fault::RenameFails);
    let first = machine.import(&card, &Import::lisbon(FLAT).backed_up());
    assert!(matches!(
        backup_outcome(report(&first, RAW), BACKUPS[0]),
        BackupOutcome::Failed(BackupError::Sidecar(_))
    ));
    assert!(!FormatReadiness::of(&first).can_be_formatted());
    card.remove_fault(Path::new(AT_BACKUP));
    // A later run writes a later xmp:MetadataDate; the backup gets the target's bytes.
    machine.clock.advance(Duration::from_secs(3600));
    let summary = machine.import(&card, &Import::lisbon(FLAT).backed_up());
    let file = report(&summary, RAW);
    assert!(
        matches!(file.outcome, FileOutcome::ImportedBefore { .. }),
        "{:?}",
        file.outcome
    );
    assert_same_at_backups(&card, "DSCF0042.RAF.xmp");
    assert!(matches!(
        backup_outcome(file, BACKUPS[0]),
        BackupOutcome::AlreadyBackedUp
    ));
    assert!(matches!(
        backup_outcome(file, BACKUPS[1]),
        BackupOutcome::BackedUpBefore
    ));
    assert!(FormatReadiness::of(&summary).can_be_formatted());
    let expected = checksum(&content(&card, SIDECAR).unwrap());
    let copies = machine.history.copies();
    assert_eq!(copies.len(), 3);
    for copy in copies {
        let recorded = copy.sidecar.map(|sidecar| sidecar.checksum);
        assert_eq!(
            recorded.as_ref(),
            Some(&expected),
            "{}",
            copy.destination.display()
        );
    }
}

/// spec: IMPORT-007.15
#[test]
fn a_backup_that_is_still_read_only_fails_again_and_a_later_run_completes_it() {
    let machine = Machine::new();
    let card = raw_card();
    let part = Path::new("/backup-1/DSCF0042.RAF.xmp.part");
    card.inject_fault(part, Fault::WriteFails { after_bytes: 0 });
    machine.import(&card, &Import::lisbon(FLAT).backed_up());
    let again = machine.import(&card, &Import::lisbon(FLAT).backed_up());
    assert!(matches!(
        backup_outcome(report(&again, RAW), BACKUPS[0]),
        BackupOutcome::Failed(BackupError::Sidecar(_))
    ));
    assert_eq!(content(&card, AT_BACKUP), None);
    assert!(!FormatReadiness::of(&again).can_be_formatted());
    let at_backup = |machine: &Machine| {
        machine
            .history
            .copies()
            .into_iter()
            .filter(|copy| copy.destination == Path::new(BACKUPS[0]))
            .count()
    };
    assert_eq!(at_backup(&machine), 0);
    card.remove_fault(part);
    let last = machine.import(&card, &Import::lisbon(FLAT).backed_up());
    assert_same_at_backups(&card, "DSCF0042.RAF.xmp");
    assert!(FormatReadiness::of(&last).can_be_formatted());
    assert_eq!(at_backup(&machine), 1);
}

/// spec: IMPORT-007.15
#[test]
fn a_file_found_already_imported_completes_the_sidecar_at_a_backup() {
    let card = raw_card();
    card.inject_fault(Path::new(AT_BACKUP), Fault::RenameFails);
    Machine::new().import(&card, &Import::lisbon(FLAT).backed_up());
    card.remove_fault(Path::new(AT_BACKUP));
    let other = Machine::new();
    other.clock.advance(Duration::from_secs(3600));
    let summary = other.import(&card, &Import::lisbon(FLAT).backed_up());
    let file = report(&summary, RAW);
    assert!(
        matches!(file.outcome, FileOutcome::AlreadyImported { .. }),
        "{:?}",
        file.outcome
    );
    assert_same_at_backups(&card, "DSCF0042.RAF.xmp");
    assert!(FormatReadiness::of(&summary).can_be_formatted());
}

/// spec: IMPORT-007.15
#[test]
fn a_dry_re_run_writes_no_sidecar_at_a_backup() {
    let machine = Machine::new();
    let card = raw_card();
    card.inject_fault(Path::new(AT_BACKUP), Fault::RenameFails);
    machine.import(&card, &Import::lisbon(FLAT).backed_up());
    card.remove_fault(Path::new(AT_BACKUP));
    let mut import = Import::lisbon(FLAT).backed_up();
    import.dry_run = true;
    machine.import(&card, &import);
    assert_eq!(content(&card, AT_BACKUP), None);
}
