// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A re-run counts a source sidecar the first run saved where it still holds what was
//! recorded, and writes nothing for it (IMPORT-020.9, IMPORT-020.10).

#![allow(clippy::unwrap_used, reason = "tests may panic")]

mod sidecar_support;

use std::path::{Path, PathBuf};

use import::{CarriedSidecar, FileOutcome, FormatBlocker, FormatReadiness};
use sidecar_support::{FLAT, Import, Machine, bytes_of, camera_packet, put, raw_card_with, report};

const RAW: &str = "DSCF0042.RAF";
const SOURCE_SIDECAR: &str = "DSCF0042.RAF.xmp";

/// spec: IMPORT-020.9
#[test]
fn a_re_run_counts_the_carried_sidecar_the_first_run_saved() {
    let machine = Machine::new();
    let sidecar = bytes_of(&machine, &camera_packet());
    let card = raw_card_with(&[(SOURCE_SIDECAR, &sidecar)]);
    let import = Import::lisbon(FLAT).backed_up();
    machine.import(&card, &import);
    let written = card
        .content(Path::new("/library/DSCF0042.RAF.xmp"))
        .unwrap();

    let summary = machine.import(&card, &import);

    let file = report(&summary, RAW);
    assert!(
        matches!(file.outcome, FileOutcome::ImportedBefore { .. }),
        "{:?}",
        file.outcome
    );
    assert_eq!(
        file.carried_sidecars,
        [CarriedSidecar {
            path: PathBuf::from(SOURCE_SIDECAR),
            target: Some(SOURCE_SIDECAR.to_owned()),
            verified_at: 3,
        }]
    );
    for location in ["/library", "/backup-1", "/backup-2"] {
        let path = Path::new(location).join(SOURCE_SIDECAR);
        assert_eq!(card.content(&path), Some(written.clone()), "{location}");
    }
    assert!(FormatReadiness::of(&summary).can_be_formatted());
}

/// spec: IMPORT-020.10
#[test]
fn a_carried_sidecar_changed_since_does_not_count_there() {
    let machine = Machine::new();
    let sidecar = bytes_of(&machine, &camera_packet());
    let card = raw_card_with(&[(SOURCE_SIDECAR, &sidecar)]);
    let import = Import::lisbon(FLAT).backed_up();
    machine.import(&card, &import);
    let edited = Path::new("/backup-1/DSCF0042.RAF.xmp");
    put(&card, edited, b"edited");

    let summary = machine.import(&card, &import);

    let readiness = FormatReadiness::of(&summary);
    assert!(!readiness.can_be_formatted());
    assert_eq!(
        readiness.blockers,
        [FormatBlocker::NotSaved {
            path: PathBuf::from(SOURCE_SIDECAR),
            verified_at: 2,
            required_at: 3,
        }]
    );
    assert_eq!(card.content(edited), Some(b"edited".to_vec()));
}

/// spec: IMPORT-020.9
#[test]
fn a_re_run_without_a_record_counts_a_sidecar_copied_unchanged() {
    let sidecar = bytes_of(&Machine::new(), &camera_packet());
    let card = raw_card_with(&[(SOURCE_SIDECAR, &sidecar)]);
    let mut import = Import::lisbon(FLAT).backed_up();
    import.job_code = None;
    Machine::new().import(&card, &import);

    // A new history knows nothing, so the run finds the file at its target.
    let summary = Machine::new().import(&card, &import);

    let file = report(&summary, RAW);
    assert!(matches!(file.outcome, FileOutcome::AlreadyImported { .. }));
    assert_eq!(file.carried_sidecars[0].verified_at, 3);
    assert!(FormatReadiness::of(&summary).can_be_formatted());
}

/// spec: IMPORT-020.10
#[test]
fn a_re_run_without_a_record_does_not_count_a_merged_sidecar() {
    let sidecar = bytes_of(&Machine::new(), &camera_packet());
    let card = raw_card_with(&[(SOURCE_SIDECAR, &sidecar)]);
    let import = Import::lisbon(FLAT);
    Machine::new().import(&card, &import);
    let written = card.content(Path::new("/library/DSCF0042.RAF.xmp"));

    let summary = Machine::new().import(&card, &import);

    assert_eq!(report(&summary, RAW).carried_sidecars[0].verified_at, 0);
    assert!(!FormatReadiness::of(&summary).can_be_formatted());
    assert_eq!(
        card.content(Path::new("/library/DSCF0042.RAF.xmp")),
        written
    );
}
