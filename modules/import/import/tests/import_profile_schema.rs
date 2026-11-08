// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The published JSON Schemas of the import profile (IMPORT-011, "JSON Schema") and of
//! `config.yaml` (IMPORT-019). `cargo xtask schemas` writes them; `cargo xtask
//! check-schemas` fails on drift too.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use std::fs;
use std::path::Path;

use import::{
    CONFIGURATION_SCHEMA_ID, IMPORT_PROFILE_SCHEMA_ID, IMPORT_PROFILE_V1_SCHEMA_ID,
    configuration_json_schema, import_profile_json_schema, import_profile_v1_json_schema,
};

fn published(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../schemas")
            .join(name),
    )
    .unwrap()
}

/// spec: IMPORT-011.10
#[test]
fn the_published_schemas_are_the_generated_ones() {
    assert_eq!(
        IMPORT_PROFILE_SCHEMA_ID,
        "https://polandy.github.io/framehive/schemas/import-profile-v2.json"
    );
    assert_eq!(
        IMPORT_PROFILE_V1_SCHEMA_ID,
        "https://polandy.github.io/framehive/schemas/import-profile-v1.json"
    );
    let generated = import_profile_json_schema();
    assert!(generated.contains(&format!("\"$id\": \"{IMPORT_PROFILE_SCHEMA_ID}\"")));
    for key in [
        "video_destination",
        "naming",
        "job_codes",
        "backup",
        "metadata",
    ] {
        assert!(generated.contains(&format!("\n    \"{key}\": {{")), "{key}");
    }
    // The folder to import into is given on the command line (ADR 0036, decision 8).
    assert!(!generated.contains("\n    \"destination\": {"));
    let version_1 = import_profile_v1_json_schema();
    assert!(!version_1.contains("\n    \"metadata\": {"));
    assert_eq!(
        published("import-profile-v2.json"),
        generated,
        "run `cargo xtask schemas`"
    );
    assert_eq!(
        published("import-profile-v1.json"),
        version_1,
        "run `cargo xtask schemas`"
    );
}

/// spec: IMPORT-019.1
#[test]
fn the_published_configuration_schema_is_the_generated_one() {
    let generated = configuration_json_schema();
    assert!(generated.contains(&format!("\"$id\": \"{CONFIGURATION_SCHEMA_ID}\"")));
    for key in ["creator", "copyright", "keywords", "sidecar_naming"] {
        assert!(generated.contains(&format!("\"{key}\": {{")), "{key}");
    }
    assert_eq!(
        published("configuration-v1.json"),
        generated,
        "run `cargo xtask schemas`"
    );
}
