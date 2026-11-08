// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import writes each photo's XMP sidecar with the job code and the `metadata`
//! values, verifies it at every location and records it (IMPORT-019).

#![allow(clippy::unwrap_used, reason = "tests may panic")]

mod sidecar_support;

use std::path::Path;
use std::time::Duration;

use files::Hasher;
use files::testing::{Fault, FnvHasher};
use import::{
    BackupError, BackupOutcome, FileError, FileOutcome, FormatReadiness, ImportMetadata, RunError,
};
use metadata::xmp::namespaces::{DUBLIN_CORE, LIGHTROOM, MWG_KEYWORDS};
use metadata::xmp::{SidecarNaming, XmpName, XmpPacket, XmpValue};
use sidecar_support::{
    DATED, DESTINATION, FLAT, Import, Machine, card_with, copied, n16_values, photoshop,
    raw_taken_at, report, sidecars_below, text,
};

const RAW: &str = "DSCF0042.RAF";
const SIDECAR: &str = "/library/DSCF0042.RAF.xmp";

fn dc(local: &str) -> XmpName {
    XmpName::new(DUBLIN_CORE, local)
}

/// The texts of the items of the array `name`.
fn items(packet: &XmpPacket, name: &XmpName) -> Vec<String> {
    match packet.property(name).map(|node| &node.value) {
        Some(XmpValue::Array(_, items)) => items
            .iter()
            .filter_map(|item| match &item.value {
                XmpValue::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn rights(packet: &XmpPacket) -> Vec<String> {
    items(packet, &dc("rights"))
}

fn checksum(content: &[u8]) -> domain::Checksum {
    let mut hash = FnvHasher.start();
    hash.update(content);
    hash.finish()
}

fn raw_card() -> files::testing::MemoryFileSystem {
    card_with(&[(RAW, &raw_taken_at("2026:10:17 14:05:09"))])
}

/// spec: IMPORT-019.1
#[test]
fn writes_the_values_of_n16() {
    let machine = Machine::new();
    let card = raw_card();
    let mut import = Import::lisbon(FLAT);
    import.metadata = n16_values();
    let summary = machine.import(&card, &import);
    assert_eq!(copied(report(&summary, RAW)), RAW);
    let sidecar = machine.sidecar(&card, SIDECAR);
    let job_code = text(&sidecar, &photoshop("TransmissionReference"));
    assert_eq!(job_code.as_deref(), Some("Lisbon"));
    assert_eq!(items(&sidecar, &dc("creator")), ["Andy Pollari"]);
    assert_eq!(rights(&sidecar), ["© 2026 Andy Pollari"]);
    assert_eq!(items(&sidecar, &dc("subject")), ["Beach"]);
    let hierarchical = XmpName::new(LIGHTROOM, "hierarchicalSubject");
    assert_eq!(items(&sidecar, &hierarchical), ["Holiday|Beach"]);
    let mut expected = XmpPacket::new();
    let levels = ["Holiday".to_owned(), "Beach".to_owned()];
    expected.add_keywords(&[&levels]).unwrap();
    let keywords = XmpName::new(MWG_KEYWORDS, "Keywords");
    assert_eq!(sidecar.property(&keywords), expected.property(&keywords));
    assert_eq!(
        report(&summary, RAW).sidecar.as_deref(),
        Some("DSCF0042.RAF.xmp")
    );
}

/// spec: IMPORT-019.2
#[test]
fn writes_no_sidecar_without_values() {
    let machine = Machine::new();
    let card = raw_card();
    let mut import = Import::lisbon(FLAT);
    import.job_code = None;
    let summary = machine.import(&card, &import);
    assert_eq!(copied(report(&summary, RAW)), RAW);
    assert_eq!(report(&summary, RAW).sidecar, None);
    assert_eq!(sidecars_below(&card, "/"), Vec::<std::path::PathBuf>::new());
    assert_eq!(machine.history.copies()[0].sidecar, None);
}

/// spec: IMPORT-019.6
#[test]
fn takes_the_year_of_the_import_date_without_a_capture_time() {
    let machine = Machine::new();
    // Modified at the end of 2025, imported on 2026-11-08.
    let card = card_with(&[("DSCF0042.JPG", b"no capture time")]);
    let mut import = Import::lisbon(FLAT);
    import.metadata = n16_values();
    machine.import(&card, &import);
    let sidecar = machine.sidecar(&card, "/library/DSCF0042.JPG.xmp");
    assert_eq!(rights(&sidecar), ["© 2026 Andy Pollari"]);
}

/// spec: IMPORT-019.6
#[test]
fn takes_the_year_of_the_capture_time() {
    let machine = Machine::new();
    let card = card_with(&[(RAW, &raw_taken_at("2025:06:01 10:00:00"))]);
    let mut import = Import::lisbon(FLAT);
    import.metadata = n16_values();
    machine.import(&card, &import);
    assert_eq!(
        rights(&machine.sidecar(&card, SIDECAR)),
        ["© 2025 Andy Pollari"]
    );
}

/// spec: IMPORT-019.7
#[test]
fn names_the_sidecar_without_the_extension() {
    let machine = Machine::new();
    let card = raw_card();
    let mut import = Import::lisbon(DATED);
    import.metadata.sidecar_naming = SidecarNaming::OriginalWithoutExtension;
    let summary = machine.import(&card, &import);
    assert_eq!(copied(report(&summary, RAW)), "20261017-0042.RAF");
    assert_eq!(
        sidecars_below(&card, DESTINATION),
        [Path::new("/library/20261017-0042.xmp")]
    );
}

/// spec: IMPORT-019.8
#[test]
fn a_raw_and_its_jpeg_get_a_sidecar_each() {
    let machine = Machine::new();
    let photo = raw_taken_at("2026:10:17 14:05:09");
    let card = card_with(&[(RAW, &photo), ("DSCF0042.JPG", b"jpeg")]);
    machine.import(&card, &Import::lisbon(FLAT));
    for path in ["/library/DSCF0042.JPG.xmp", SIDECAR] {
        let job_code = text(
            &machine.sidecar(&card, path),
            &photoshop("TransmissionReference"),
        );
        assert_eq!(job_code.as_deref(), Some("Lisbon"), "{path}");
    }
}

/// spec: IMPORT-019.9
#[test]
fn a_video_gets_no_sidecar() {
    let machine = Machine::new();
    let card = card_with(&[("clip.MOV", b"video")]);
    let summary = machine.import(&card, &Import::lisbon(FLAT));
    assert_eq!(copied(report(&summary, "clip.MOV")), "clip.MOV");
    assert_eq!(report(&summary, "clip.MOV").sidecar, None);
    assert_eq!(sidecars_below(&card, "/"), Vec::<std::path::PathBuf>::new());
}

/// spec: IMPORT-019.14
#[test]
fn each_backup_gets_the_same_sidecar() {
    let machine = Machine::new();
    let card = raw_card();
    let summary = machine.import(&card, &Import::lisbon(FLAT).backed_up());
    let written = card.content(Path::new(SIDECAR)).unwrap();
    for backup in ["/backup-1/DSCF0042.RAF.xmp", "/backup-2/DSCF0042.RAF.xmp"] {
        assert_eq!(
            card.content(Path::new(backup)).as_ref(),
            Some(&written),
            "{backup}"
        );
    }
    for counts in &summary.backups {
        assert_eq!((counts.backed_up, counts.failed), (1, 0), "{counts:?}");
    }
}

/// spec: IMPORT-019.15
#[test]
fn records_the_sidecar_at_every_location() {
    let machine = Machine::new();
    let card = raw_card();
    machine.import(&card, &Import::lisbon(FLAT).backed_up());
    let expected = checksum(&card.content(Path::new(SIDECAR)).unwrap());
    let copies = machine.history.copies();
    assert_eq!(copies.len(), 3);
    for copy in copies {
        let sidecar = copy.sidecar.unwrap();
        assert_eq!(sidecar.target, "DSCF0042.RAF.xmp");
        assert_eq!(sidecar.checksum, expected, "{}", copy.destination.display());
    }
}

/// spec: IMPORT-019.16
#[test]
fn a_failed_sidecar_fails_its_file() {
    let machine = Machine::new();
    let card = raw_card();
    card.inject_fault(Path::new(SIDECAR), Fault::RenameFails);
    let summary = machine.import(&card, &Import::lisbon(FLAT));
    let FileOutcome::Failed(FileError::Sidecar(error)) = &report(&summary, RAW).outcome else {
        panic!(
            "a sidecar failure expected: {:?}",
            report(&summary, RAW).outcome
        );
    };
    assert_eq!(error.path, Path::new(SIDECAR));
    assert!(error.to_string().contains(SIDECAR), "{error}");
    assert!(card.content(Path::new("/library/DSCF0042.RAF")).is_some());
    assert!(!FormatReadiness::of(&summary).can_be_formatted());
    assert!(machine.history.copies().is_empty());
}

/// spec: IMPORT-019.18
#[test]
fn a_re_run_completes_a_failed_sidecar() {
    let machine = Machine::new();
    let card = raw_card();
    card.inject_fault(Path::new(SIDECAR), Fault::RenameFails);
    machine.import(&card, &Import::lisbon(FLAT));
    card.remove_fault(Path::new(SIDECAR));
    let summary = machine.import(&card, &Import::lisbon(FLAT));
    let file = report(&summary, RAW);
    assert!(
        matches!(&file.outcome, FileOutcome::AlreadyImported { target } if target == RAW),
        "{:?}",
        file.outcome
    );
    assert_eq!(file.sidecar.as_deref(), Some("DSCF0042.RAF.xmp"));
    let packet = machine.sidecar(&card, SIDECAR);
    assert_eq!(
        text(&packet, &photoshop("TransmissionReference")).as_deref(),
        Some("Lisbon")
    );
    let copies = machine.history.copies();
    assert_eq!(copies.len(), 1);
    let expected = checksum(&card.content(Path::new(SIDECAR)).unwrap());
    assert_eq!(copies[0].sidecar.as_ref().unwrap().checksum, expected);
    assert!(FormatReadiness::of(&summary).can_be_formatted());
}

/// spec: IMPORT-019.18
#[test]
fn a_re_run_completes_a_failed_sidecar_at_a_backup_and_keeps_the_others() {
    let machine = Machine::new();
    let card = raw_card();
    let at_backups = ["/backup-1/DSCF0042.RAF.xmp", "/backup-2/DSCF0042.RAF.xmp"].map(Path::new);
    card.inject_fault(Path::new(SIDECAR), Fault::RenameFails);
    card.inject_fault(at_backups[0], Fault::RenameFails);
    machine.import(&card, &Import::lisbon(FLAT).backed_up());
    let kept = card.content(at_backups[1]).unwrap();
    card.remove_fault(Path::new(SIDECAR));
    card.remove_fault(at_backups[0]);
    // A later run writes a later xmp:MetadataDate, so the kept sidecar differs.
    machine.clock.advance(Duration::from_secs(3600));
    let summary = machine.import(&card, &Import::lisbon(FLAT).backed_up());
    let file = report(&summary, RAW);
    assert!(matches!(file.outcome, FileOutcome::AlreadyImported { .. }));
    let written = card.content(Path::new(SIDECAR)).unwrap();
    assert_ne!(written, kept);
    assert_eq!(card.content(at_backups[0]), Some(written.clone()));
    assert_eq!(card.content(at_backups[1]), Some(kept));
    assert!(FormatReadiness::of(&summary).can_be_formatted());
    let copies = machine.history.copies();
    assert_eq!(copies.len(), 3);
    for copy in copies {
        let recorded = copy.sidecar.map(|sidecar| sidecar.checksum);
        let expected = (copy.destination != Path::new("/backup-2")).then(|| checksum(&written));
        assert_eq!(recorded, expected, "{}", copy.destination.display());
    }
}

#[test]
fn a_dry_run_lists_the_sidecar_a_re_run_would_complete() {
    let machine = Machine::new();
    let card = raw_card();
    card.inject_fault(Path::new(SIDECAR), Fault::RenameFails);
    machine.import(&card, &Import::lisbon(FLAT));
    card.remove_fault(Path::new(SIDECAR));
    let mut import = Import::lisbon(FLAT);
    import.dry_run = true;
    let summary = machine.import(&card, &import);
    assert_eq!(
        report(&summary, RAW).sidecar.as_deref(),
        Some("DSCF0042.RAF.xmp")
    );
    assert!(sidecars_below(&card, "/").is_empty());
}

#[test]
fn a_sidecar_that_fails_at_a_backup_fails_that_backup() {
    let machine = Machine::new();
    let card = raw_card();
    card.inject_fault(Path::new("/backup-1/DSCF0042.RAF.xmp"), Fault::RenameFails);
    let summary = machine.import(&card, &Import::lisbon(FLAT).backed_up());
    let file = report(&summary, RAW);
    assert_eq!(copied(file), RAW);
    assert!(matches!(
        &file.backups[0].outcome,
        BackupOutcome::Failed(BackupError::Sidecar(_))
    ));
    assert!(matches!(file.backups[1].outcome, BackupOutcome::BackedUp));
    assert!(!FormatReadiness::of(&summary).can_be_formatted());
}

/// spec: IMPORT-019.17
#[test]
fn a_dry_run_lists_the_sidecar_and_writes_nothing() {
    let machine = Machine::new();
    let card = raw_card();
    let mut import = Import::lisbon(FLAT);
    import.dry_run = true;
    let summary = machine.import(&card, &import);
    let file = report(&summary, RAW);
    assert!(matches!(&file.outcome, FileOutcome::WouldCopy { target } if target == RAW));
    assert_eq!(file.sidecar.as_deref(), Some("DSCF0042.RAF.xmp"));
    assert_eq!(sidecars_below(&card, "/"), Vec::<std::path::PathBuf>::new());
}

#[test]
fn a_run_that_writes_sidecars_needs_an_xml_syntax() {
    let machine = Machine::new();
    let card = raw_card();
    let error = machine
        .import_without_xml(&card, &Import::lisbon(FLAT))
        .unwrap_err();
    assert!(
        matches!(error, RunError::SidecarsWithoutXmlSyntax),
        "{error}"
    );
    let mut without_values = Import::lisbon(FLAT);
    without_values.job_code = None;
    without_values.metadata = ImportMetadata::default();
    assert!(machine.import_without_xml(&card, &without_values).is_ok());
}
