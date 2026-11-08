// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` with the `metadata` of `config.yaml` (IMPORT-019) and its report
//! of sidecars (CLI-017).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::fs;
use std::path::Path;
use std::sync::Arc;

use adapter_yaml::SaphyrParser;
use files::testing::{MemoryCameras, MemoryFreeSpace};
use import::{Configuration, FileOutcome};

use super::tests::{Folders, Run};
use crate::testing::CAMERA_SIDECAR;

/// The end of the line of `DSCF0001.RAF`, which has no capture time to read.
const DATED: &str = " (dated by modification time: format not read)";

/// The target of the card's only file.
fn target(run: &Run) -> String {
    let Ok(summary) = &run.result else {
        panic!("the import did not run: {}", run.errors);
    };
    match &summary.files[0].outcome {
        FileOutcome::Copied { target, .. } | FileOutcome::WouldCopy { target } => target.clone(),
        outcome => panic!("not copied: {outcome:?}"),
    }
}

/// spec: IMPORT-019.1
#[test]
fn the_metadata_of_config_yaml_reaches_the_sidecar_of_the_import() {
    let mut folders = Folders::new();
    folders.configuration = Configuration::read(
        &SaphyrParser::new(),
        "config.yaml",
        "schema_version: 1\nmetadata:\n  creator: [Andy Pollari]\n",
    )
    .unwrap();
    let library = folders.path("library");

    let run = folders.import_from(
        folders.path("card").as_os_str(),
        &[&library],
        Arc::new(MemoryFreeSpace::unlimited()),
        &MemoryCameras::new(),
    );

    let Ok(summary) = &run.result else {
        panic!("the import did not run: {}", run.errors);
    };
    assert!(!summary.has_failures(), "{}", run.output);
    let sidecar = summary.files[0].sidecar.as_ref().unwrap();
    let written = fs::read_to_string(library.join(sidecar)).unwrap();
    assert!(written.contains("Andy Pollari"), "{written}");
}

/// Folders whose `config.yaml` has the creator `Andy Pollari`.
fn folders_with_creator() -> Folders {
    let mut folders = Folders::new();
    folders.configuration = Configuration::read(
        &SaphyrParser::new(),
        "config.yaml",
        "schema_version: 1\nmetadata:\n  creator: [Andy Pollari]\n",
    )
    .unwrap();
    folders
}

/// Runs `framehive import <card> --destination <library> <arguments>`.
fn import(folders: &Folders, arguments: &[&str]) -> Run {
    let library = folders.path("library");
    let mut line: Vec<&Path> = vec![&library];
    line.extend(arguments.iter().map(Path::new));
    folders.import_from(
        folders.path("card").as_os_str(),
        &line,
        Arc::new(MemoryFreeSpace::unlimited()),
        &MemoryCameras::new(),
    )
}

/// spec: CLI-017.1
#[test]
fn a_written_sidecar_is_reported_and_counted() {
    let folders = folders_with_creator();

    let run = import(&folders, &[]);

    let target = target(&run);
    assert!(
        run.output.contains(&format!(
            "copied  DCIM/100_FUJI/DSCF0001.RAF -> {target}{DATED}\n\
             \x20 wrote sidecar  {target}.xmp\n"
        )),
        "{}",
        run.output
    );
    assert!(
        run.output
            .contains("0 skipped, 0 failed, 0 left out\n1 sidecars written\n"),
        "{}",
        run.output
    );
}

/// spec: CLI-017.2
#[test]
fn a_dry_run_lists_the_sidecars_it_would_write() {
    let folders = folders_with_creator();

    let run = import(&folders, &["--dry-run"]);

    let target = target(&run);
    assert!(
        run.output.contains(&format!(
            "would copy  DCIM/100_FUJI/DSCF0001.RAF -> {target}{DATED}\n\
             \x20 would write sidecar  {target}.xmp\n"
        )),
        "{}",
        run.output
    );
    assert!(!run.output.contains("sidecars written"), "{}", run.output);
    assert!(!folders.path("library").exists());
}

/// spec: CLI-017.3
#[test]
fn a_carried_along_sidecar_reports_the_replaced_and_kept_values() {
    let folders = folders_with_creator();
    fs::write(
        folders.path("card/DCIM/100_FUJI/DSCF0001.RAF.xmp"),
        CAMERA_SIDECAR,
    )
    .unwrap();

    let run = import(&folders, &["--job-code", "Lisbon"]);

    let target = target(&run);
    assert!(
        run.output.contains(&format!(
            "copied  DCIM/100_FUJI/DSCF0001.RAF -> {target}{DATED}\n\
             \x20 wrote sidecar  DCIM/100_FUJI/DSCF0001.RAF.xmp -> {target}.xmp\n\
             \x20 replaced job code  Porto -> Lisbon\n\
             \x20 kept creator  Camera Owner (not Andy Pollari)\n"
        )),
        "{}\n{}",
        run.output,
        run.errors
    );
}

/// spec: CLI-017.4
#[test]
fn without_values_the_report_has_no_sidecar_line() {
    let folders = Folders::new();

    let run = import(&folders, &[]);

    assert!(!run.output.contains("sidecar"), "{}", run.output);
}
