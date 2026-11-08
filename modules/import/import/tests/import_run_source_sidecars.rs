// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A source `.xmp` sidecar comes with its file, with the import's values merged in
//! (IMPORT-020).

#![allow(clippy::unwrap_used, reason = "tests may panic")]

mod sidecar_support;

use std::path::{Path, PathBuf};

use files::testing::{Fault, MemoryFileSystem};
use import::{
    CarriedSidecar, ExistingValue, ExistingValueResult, FormatBlocker, FormatReadiness,
    ImportMetadata, Keyword, RunError, SidecarField,
};
use metadata::xmp::namespaces::{DUBLIN_CORE, PHOTOSHOP};
use metadata::xmp::{XmpName, XmpPacket, XmpValue, serialize_xmp};
use sidecar_support::{
    DATED, DESTINATION, FLAT, Import, Machine, card_with, copied, put, raw_taken_at, report,
    sidecars_below, text,
};

const RAW: &str = "DSCF0042.RAF";
const SOURCE_SIDECAR: &str = "DSCF0042.RAF.xmp";
const UNKNOWN: &str = "http://example.com/camera/1.0/";

fn dc(local: &str) -> XmpName {
    XmpName::new(DUBLIN_CORE, local)
}

fn job_code() -> XmpName {
    XmpName::new(PHOTOSHOP, "TransmissionReference")
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

/// A source sidecar with a property of a namespace no standard defines.
fn camera_packet() -> XmpPacket {
    let mut packet = XmpPacket::new();
    packet
        .set_text(&XmpName::new(UNKNOWN, "FilmMode"), "Classic Chrome")
        .unwrap();
    packet
}

/// The bytes of `packet` as the machine's XML syntax writes them.
fn bytes_of(machine: &Machine, packet: &XmpPacket) -> Vec<u8> {
    serialize_xmp(packet, &machine.clock, &machine.xml)
        .unwrap()
        .into_bytes()
}

/// A card with `DSCF0042.RAF` captured on 2026-10-17 and `extra` files.
fn raw_card_with(extra: &[(&str, &[u8])]) -> MemoryFileSystem {
    let raw = raw_taken_at("2026:10:17 14:05:09");
    let mut files: Vec<(&str, &[u8])> = vec![(RAW, &raw)];
    files.extend_from_slice(extra);
    card_with(&files)
}

fn values(creator: &str, copyright: &str) -> ImportMetadata {
    ImportMetadata {
        creator: vec![creator.to_owned()],
        copyright: Some(copyright.to_owned()),
        ..ImportMetadata::default()
    }
}

/// spec: IMPORT-020.1
#[test]
fn a_source_sidecar_comes_with_its_file() {
    let machine = Machine::new();
    let sidecar = bytes_of(&machine, &camera_packet());
    let card = raw_card_with(&[(SOURCE_SIDECAR, &sidecar)]);

    let summary = machine.import(&card, &Import::lisbon(DATED));

    let file = report(&summary, RAW);
    assert_eq!(copied(file), "20261017-0042.RAF");
    assert_eq!(file.sidecar.as_deref(), Some("20261017-0042.RAF.xmp"));
    assert_eq!(
        file.carried_sidecars,
        [CarriedSidecar {
            path: PathBuf::from(SOURCE_SIDECAR),
            target: Some("20261017-0042.RAF.xmp".to_owned()),
            verified_at: 1,
        }]
    );
    assert_eq!(
        sidecars_below(&card, DESTINATION),
        [Path::new("/library/20261017-0042.RAF.xmp")]
    );
    let written = machine.sidecar(&card, "/library/20261017-0042.RAF.xmp");
    let film_mode = XmpName::new(UNKNOWN, "FilmMode");
    assert_eq!(
        text(&written, &film_mode).as_deref(),
        Some("Classic Chrome")
    );
    assert_eq!(text(&written, &job_code()).as_deref(), Some("Lisbon"));
    assert!(FormatReadiness::of(&summary).can_be_formatted());
    let recorded = machine.history.copies()[0].sidecar.clone().unwrap();
    assert_eq!(recorded.target, "20261017-0042.RAF.xmp");
}

/// spec: IMPORT-020.2
#[test]
fn a_sidecar_without_extension_goes_to_the_raw() {
    let machine = Machine::new();
    let sidecar = bytes_of(&machine, &camera_packet());
    let card = raw_card_with(&[("DSCF0042.JPG", b"jpeg"), ("DSCF0042.xmp", &sidecar)]);

    let summary = machine.import(&card, &Import::lisbon(DATED));

    let raw = report(&summary, RAW);
    assert_eq!(raw.carried_sidecars[0].path, Path::new("DSCF0042.xmp"));
    assert_eq!(
        raw.carried_sidecars[0].target.as_deref(),
        Some("20261017-0042.xmp")
    );
    // The values go into the sidecar that came along, in its own naming form.
    assert_eq!(raw.sidecar.as_deref(), Some("20261017-0042.xmp"));
    assert!(report(&summary, "DSCF0042.JPG").carried_sidecars.is_empty());
    let written = machine.sidecar(&card, "/library/20261017-0042.xmp");
    assert_eq!(text(&written, &job_code()).as_deref(), Some("Lisbon"));
    assert!(
        sidecars_below(&card, DESTINATION).contains(&PathBuf::from("/library/20261017-0042.xmp"))
    );
    assert!(
        !sidecars_below(&card, DESTINATION)
            .contains(&PathBuf::from("/library/20261017-0042.RAF.xmp"))
    );
}

/// spec: IMPORT-020.5
#[test]
fn a_videos_source_sidecar_is_copied_unchanged() {
    let machine = Machine::new();
    let source = b"<x:xmpmeta>written by the camera</x:xmpmeta>".as_slice();
    let card = card_with(&[("clip.MOV", b"video"), ("clip.MOV.xmp", source)]);

    let summary = machine.import(&card, &Import::lisbon(FLAT));

    let file = report(&summary, "clip.MOV");
    assert_eq!(file.sidecar, None);
    assert_eq!(file.carried_sidecars[0].verified_at, 1);
    let target = card.content(Path::new("/library/clip.MOV.xmp")).unwrap();
    assert_eq!(target, source);
}

/// spec: IMPORT-020.6
#[test]
fn keywords_are_unioned() {
    let machine = Machine::new();
    let mut packet = XmpPacket::new();
    packet.add_to_bag(&dc("subject"), &["Sea"]).unwrap();
    let sidecar = bytes_of(&machine, &packet);
    let card = raw_card_with(&[(SOURCE_SIDECAR, &sidecar)]);
    let mut import = Import::lisbon(FLAT);
    import.metadata.keywords = vec![Keyword::parse("Beach").unwrap()];

    machine.import(&card, &import);

    let written = machine.sidecar(&card, "/library/DSCF0042.RAF.xmp");
    assert_eq!(items(&written, &dc("subject")), ["Sea", "Beach"]);
}

/// spec: IMPORT-020.7
#[test]
fn the_job_code_replaces_and_creator_and_copyright_are_kept() {
    let machine = Machine::new();
    let mut packet = XmpPacket::new();
    packet.set_text(&job_code(), "Porto").unwrap();
    packet
        .replace_seq(&dc("creator"), &["Camera Owner"])
        .unwrap();
    packet
        .set_language(&dc("rights"), "x-default", "© Camera Owner")
        .unwrap();
    let sidecar = bytes_of(&machine, &packet);
    let card = raw_card_with(&[(SOURCE_SIDECAR, &sidecar)]);
    let mut import = Import::lisbon(FLAT);
    import.metadata = values("Andy Pollari", "© {year} Andy Pollari");

    let summary = machine.import(&card, &import);

    let written = machine.sidecar(&card, "/library/DSCF0042.RAF.xmp");
    assert_eq!(text(&written, &job_code()).as_deref(), Some("Lisbon"));
    assert_eq!(items(&written, &dc("creator")), ["Camera Owner"]);
    assert_eq!(items(&written, &dc("rights")), ["© Camera Owner"]);
    let existing = |field, existing: &str, import: &str, result| ExistingValue {
        field,
        existing: existing.to_owned(),
        import: import.to_owned(),
        result,
    };
    assert_eq!(
        report(&summary, RAW).existing_values,
        [
            existing(
                SidecarField::JobCode,
                "Porto",
                "Lisbon",
                ExistingValueResult::Replaced
            ),
            existing(
                SidecarField::Creator,
                "Camera Owner",
                "Andy Pollari",
                ExistingValueResult::Kept
            ),
            existing(
                SidecarField::Copyright,
                "© Camera Owner",
                "© 2026 Andy Pollari",
                ExistingValueResult::Kept
            ),
        ]
    );
}

/// spec: IMPORT-020.8
#[test]
fn a_carried_sidecar_failing_at_a_backup_keeps_the_card() {
    let machine = Machine::new();
    let sidecar = bytes_of(&machine, &camera_packet());
    let card = raw_card_with(&[(SOURCE_SIDECAR, &sidecar)]);
    card.inject_fault(Path::new("/backup-2/DSCF0042.RAF.xmp"), Fault::RenameFails);

    let summary = machine.import(&card, &Import::lisbon(FLAT).backed_up());

    let readiness = FormatReadiness::of(&summary);
    assert!(!readiness.can_be_formatted());
    let sidecar_blocker = FormatBlocker::NotSaved {
        path: PathBuf::from(SOURCE_SIDECAR),
        verified_at: 2,
        required_at: 3,
    };
    assert!(
        readiness.blockers.contains(&sidecar_blocker),
        "{:?}",
        readiness.blockers
    );
}

#[test]
fn a_carried_sidecar_without_values_is_copied_unchanged_to_every_location() {
    let machine = Machine::new();
    let sidecar = bytes_of(&machine, &camera_packet());
    let card = raw_card_with(&[(SOURCE_SIDECAR, &sidecar)]);
    let mut import = Import::lisbon(FLAT).backed_up();
    import.job_code = None;

    let summary = machine.import(&card, &import);

    for location in ["/library", "/backup-1", "/backup-2"] {
        let path = Path::new(location).join(SOURCE_SIDECAR);
        assert_eq!(card.content(&path), Some(sidecar.clone()), "{location}");
    }
    assert_eq!(report(&summary, RAW).carried_sidecars[0].verified_at, 3);
    assert!(FormatReadiness::of(&summary).can_be_formatted());
}

#[test]
fn a_carried_sidecar_takes_its_files_suffix() {
    let machine = Machine::new();
    let sidecar = bytes_of(&machine, &camera_packet());
    let card = raw_card_with(&[("DSCF0042.xmp", &sidecar)]);
    put(&card, Path::new("/library/20261017-0042.xmp"), b"leftover");

    let summary = machine.import(&card, &Import::lisbon(DATED));

    let file = report(&summary, RAW);
    assert_eq!(copied(file), "20261017-0042-1.RAF");
    assert_eq!(
        file.carried_sidecars[0].target.as_deref(),
        Some("20261017-0042-1.xmp")
    );
    assert_eq!(
        card.content(Path::new("/library/20261017-0042.xmp")),
        Some(b"leftover".to_vec())
    );
}

#[test]
fn a_dry_run_names_the_carried_sidecar_and_writes_nothing() {
    let machine = Machine::new();
    let sidecar = bytes_of(&machine, &camera_packet());
    let card = raw_card_with(&[(SOURCE_SIDECAR, &sidecar)]);
    let mut import = Import::lisbon(DATED);
    import.dry_run = true;

    let summary = machine.import(&card, &import);

    let carried = &report(&summary, RAW).carried_sidecars[0];
    assert_eq!(carried.target.as_deref(), Some("20261017-0042.RAF.xmp"));
    assert_eq!(carried.verified_at, 0);
    assert!(sidecars_below(&card, DESTINATION).is_empty());
}

#[test]
fn a_carried_sidecar_needs_the_xml_syntax() {
    let machine = Machine::new();
    let sidecar = bytes_of(&machine, &camera_packet());
    let card = raw_card_with(&[(SOURCE_SIDECAR, &sidecar)]);
    let mut import = Import::lisbon(FLAT);
    import.job_code = None;

    let result = machine.import_without_xml(&card, &import);

    assert!(matches!(result, Err(RunError::SidecarsWithoutXmlSyntax)));
}
