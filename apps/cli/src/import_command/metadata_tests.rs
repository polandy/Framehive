// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` with the `metadata` of `config.yaml` (IMPORT-019).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::fs;
use std::sync::Arc;

use adapter_yaml::SaphyrParser;
use files::testing::{MemoryCameras, MemoryFreeSpace};
use import::Configuration;

use super::tests::Folders;

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
