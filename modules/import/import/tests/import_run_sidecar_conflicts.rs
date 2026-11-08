// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A sidecar shares its file's conflict outcome, and a file already imported writes
//! none (IMPORT-019, "Conflicts").

#![allow(clippy::unwrap_used, reason = "tests may panic")]

mod sidecar_support;

use std::path::Path;

use import::{ConflictPolicy, FileOutcome};
use metadata::xmp::SidecarNaming;
use sidecar_support::{
    DATED, DESTINATION, FLAT, Import, Machine, card_with, copied, put, raw_taken_at, report,
    sidecars_below,
};

const RAW: &str = "DSCF0042.RAF";

fn raw_card() -> files::testing::MemoryFileSystem {
    card_with(&[(RAW, &raw_taken_at("2026:10:17 14:05:09"))])
}

/// spec: IMPORT-019.10
#[test]
fn a_suffix_goes_to_the_sidecar_too() {
    let machine = Machine::new();
    let card = raw_card();
    put(&card, Path::new("/library/20261017-0042.RAF"), b"other");
    let summary = machine.import(&card, &Import::lisbon(DATED));
    let file = report(&summary, RAW);
    assert_eq!(copied(file), "20261017-0042-1.RAF");
    assert_eq!(file.sidecar.as_deref(), Some("20261017-0042-1.RAF.xmp"));
    assert_eq!(
        sidecars_below(&card, DESTINATION),
        [Path::new("/library/20261017-0042-1.RAF.xmp")]
    );
}

/// spec: IMPORT-019.10
#[test]
fn a_raw_and_its_jpeg_without_extension_do_not_share_a_sidecar() {
    let machine = Machine::new();
    let photo = raw_taken_at("2026:10:17 14:05:09");
    let card = card_with(&[(RAW, &photo), ("DSCF0042.JPG", b"jpeg")]);
    let mut import = Import::lisbon(FLAT);
    import.metadata.sidecar_naming = SidecarNaming::OriginalWithoutExtension;
    let summary = machine.import(&card, &import);
    assert_eq!(copied(report(&summary, "DSCF0042.JPG")), "DSCF0042.JPG");
    assert_eq!(copied(report(&summary, RAW)), "DSCF0042-1.RAF");
    assert_eq!(
        sidecars_below(&card, DESTINATION),
        [
            Path::new("/library/DSCF0042-1.xmp"),
            Path::new("/library/DSCF0042.xmp")
        ]
    );
}

/// spec: IMPORT-019.11
#[test]
fn a_leftover_sidecar_at_the_target_is_not_merged() {
    let machine = Machine::new();
    let card = raw_card();
    let leftover = Path::new("/library/20261017-0042.RAF.xmp");
    put(&card, leftover, b"leftover");
    let summary = machine.import(&card, &Import::lisbon(DATED));
    let file = report(&summary, RAW);
    assert_eq!(copied(file), "20261017-0042-1.RAF");
    assert_eq!(file.sidecar.as_deref(), Some("20261017-0042-1.RAF.xmp"));
    assert_eq!(card.content(leftover), Some(b"leftover".to_vec()));
}

/// spec: IMPORT-019.12
#[test]
fn a_skipped_file_writes_no_sidecar() {
    let machine = Machine::new();
    let card = raw_card();
    put(&card, Path::new("/library/DSCF0042.RAF"), b"other");
    let mut import = Import::lisbon(FLAT);
    import.policy = ConflictPolicy::Skip;
    let summary = machine.import(&card, &import);
    let file = report(&summary, RAW);
    assert!(matches!(
        file.outcome,
        FileOutcome::SkippedNameConflict { .. }
    ));
    assert_eq!(file.sidecar, None);
    assert_eq!(sidecars_below(&card, "/"), Vec::<std::path::PathBuf>::new());
}

/// spec: IMPORT-019.13
#[test]
fn a_file_already_imported_writes_no_sidecar() {
    let card = raw_card();
    let first = Machine::new();
    first.import(&card, &Import::lisbon(FLAT));
    let sidecar = Path::new("/library/DSCF0042.RAF.xmp");
    put(&card, sidecar, b"edited since");
    // The same machine knows the file from its history, another one finds it there.
    for machine in [&first, &Machine::new()] {
        let summary = machine.import(&card, &Import::lisbon(FLAT));
        let file = report(&summary, RAW);
        assert!(
            matches!(
                file.outcome,
                FileOutcome::AlreadyImported { .. } | FileOutcome::ImportedBefore { .. }
            ),
            "{:?}",
            file.outcome
        );
        assert_eq!(file.sidecar, None);
    }
    assert_eq!(card.content(sidecar), Some(b"edited since".to_vec()));
    assert_eq!(sidecars_below(&card, "/"), [sidecar]);
}
