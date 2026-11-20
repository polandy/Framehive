// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! One run of the guided import per spec against the binary with its real adapters, on
//! a pseudo-terminal, with a card listed in a mount table file (ADR 0050).

#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::Path;

use support::guided_card::{Machine, text};

/// A sidecar on the card with the creator `Camera Owner`.
const CAMERA_SIDECAR: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:creator><rdf:Seq><rdf:li>Camera Owner</rdf:li></rdf:Seq></dc:creator></rdf:Description></rdf:RDF></x:xmpmeta>"#;

/// spec: CLI-003.9
#[test]
fn a_second_run_asks_only_for_the_job_code() {
    let machine = Machine::new();
    machine.add_pictures(&["DSCF0001.JPG"]);
    let library = text(&machine.library());

    let first = machine.run(&[&library, "2", "", "Harbour", "", "n"]);
    machine.add_pictures(&["DSCF0002.JPG"]);
    let second = machine.run(&["", "", "n"]);

    assert!(
        first.output.contains("Where should the photos go?"),
        "{}",
        first.output
    );
    assert!(
        first.output.contains("\n1 copied, 0 already imported"),
        "{}",
        first.output
    );
    assert!(
        !second.output.contains("Where should the photos go?"),
        "{}",
        second.output
    );
    assert!(
        !second.output.contains("How should the photos be named?"),
        "{}",
        second.output
    );
    assert!(second.output.contains("  1  Harbour"), "{}", second.output);
    assert!(
        second.output.contains("Choose 1 or type a job code [1]: "),
        "{}",
        second.output
    );
    assert!(
        second.output.contains("\n1 copied, 1 already imported"),
        "{}",
        second.output
    );
    for name in ["DSCF0001.JPG", "DSCF0002.JPG"] {
        assert!(
            machine
                .library()
                .join("2026/2026-10-17_Harbour")
                .join(name)
                .is_file()
        );
    }
}

/// spec: CLI-006.4
#[test]
fn a_backup_destination_gets_every_file() {
    let machine = Machine::new();
    machine.add_pictures(&["DSCF0001.JPG", "DSCF0002.JPG"]);
    let backup = text(&machine.backup());

    let run = machine.run(&[&text(&machine.library()), "", "", "b", &backup, "", "n"]);

    assert!(
        run.output.contains(&format!("  backup    {backup}\n")),
        "{}",
        run.output
    );
    assert!(
        run.output.contains(&format!(
            "backup {backup}: 2 backed up, 0 already backed up, 0 failed\n"
        )),
        "{}",
        run.output
    );
    for name in ["DSCF0001.JPG", "DSCF0002.JPG"] {
        let target = Path::new("2026/2026-10-17").join(name);
        assert!(machine.library().join(&target).is_file());
        assert!(machine.backup().join(&target).is_file());
    }
}

/// spec: CLI-011.12
#[test]
fn a_backup_on_the_file_system_of_the_destination_is_warned_about() {
    let machine = Machine::new();
    machine.add_pictures(&["DSCF0001.JPG"]);
    let (library, backup) = (text(&machine.library()), text(&machine.backup()));

    let run = machine.run(&[&library, "", "", "b", &backup, "", "n"]);

    let warning = format!(
        "backup {backup} is on the same file system as {library}; it does not protect against losing that disk"
    );
    assert_eq!(run.errors.matches(&warning).count(), 1, "{}", run.errors);
    let target = Path::new("2026/2026-10-17/DSCF0001.JPG");
    assert!(machine.library().join(target).is_file());
    assert!(machine.backup().join(target).is_file());
}

/// spec: CLI-017.6
#[test]
fn the_report_shows_a_kept_creator_and_the_sidecar_count() {
    let machine = Machine::new();
    machine.add_pictures(&["DSCF0001.JPG"]);
    machine.add_to_card("DCIM/100_FUJI/DSCF0001.JPG.xmp", CAMERA_SIDECAR.as_bytes());
    let configuration = machine.folders.configuration_folder();
    fs::create_dir_all(&configuration).unwrap();
    fs::write(
        configuration.join("config.yaml"),
        "schema_version: 1\nmetadata:\n  creator: [Andy Pollari]\n",
    )
    .unwrap();

    let run = machine.run(&[&text(&machine.library()), "", "", "", "n"]);

    assert!(
        run.output.contains(
            "copied  DCIM/100_FUJI/DSCF0001.JPG -> 2026/2026-10-17/DSCF0001.JPG\n  \
             kept creator  Camera Owner (not Andy Pollari)\n"
        ),
        "{}",
        run.output
    );
    assert!(
        run.output.contains("0 left out\n1 sidecars written\n"),
        "{}",
        run.output
    );
    assert!(
        machine
            .library()
            .join("2026/2026-10-17/DSCF0001.JPG.xmp")
            .is_file()
    );
}
