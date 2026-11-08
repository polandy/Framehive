// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive config init` (CLI-010) against the real binary, with the home folder in a
//! temporary folder.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::Path;

use adapter_yaml::SaphyrParser;
use import::{IMPORT_PROFILE_SCHEMA_ID, ImportProfile};
use support::{Folders, stderr, stdout};

/// Every key of IMPORT-011 and IMPORT-019, as it is written in a file.
const KEYS: [&str; 14] = [
    "schema_version:",
    "video_destination:",
    "naming:",
    "template:",
    "on_conflict:",
    "job_codes:",
    "backup:",
    "destinations:",
    "video_destinations:",
    "metadata:",
    "creator:",
    "copyright:",
    "keywords:",
    "sidecar_naming:",
];

fn read(text: &str) -> ImportProfile {
    ImportProfile::read(
        &SaphyrParser::new(),
        "example.yaml",
        text,
        Path::new("/home/ada"),
    )
    .unwrap()
}

/// spec: CLI-010.15
#[test]
fn config_init_writes_the_example() {
    let folders = Folders::new();
    let example = folders.profiles_folder().join("example.yaml");

    let output = folders.framehive(&["config", "init"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), format!("wrote {}\n", example.display()));
    let text = fs::read_to_string(&example).unwrap();
    assert_eq!(read(&text), read("schema_version: 2\n"));
    for key in KEYS {
        assert!(text.contains(key), "{key} is missing:\n{text}");
    }
    // A profile no longer names the folder to import into (IMPORT-011).
    assert!(!text.contains("# destination:"), "{text}");
    assert!(text.contains(&format!(
        "# yaml-language-server: $schema={IMPORT_PROFILE_SCHEMA_ID}"
    )));
}

/// spec: CLI-010.16
#[test]
fn config_init_keeps_an_existing_example() {
    let folders = Folders::new();
    let example = folders.write_profile("example", b"# mine");

    let output = folders.framehive(&["config", "init"]);

    assert_eq!(fs::read_to_string(&example).unwrap(), "# mine");
    assert_eq!(
        stderr(&output),
        format!(
            "error: {} exists already; nothing was written\n",
            example.display()
        )
    );
    assert!(stdout(&output).is_empty());
    assert_eq!(output.status.code(), Some(1));
}
