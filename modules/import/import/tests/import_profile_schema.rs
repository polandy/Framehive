// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The published JSON Schema of the import profile (IMPORT-011, "JSON Schema").
//! `cargo xtask schemas` writes it; `cargo xtask check-schemas` fails on drift too.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use std::fs;
use std::path::Path;

use import::{IMPORT_PROFILE_SCHEMA_ID, import_profile_json_schema};

/// spec: IMPORT-011.10
#[test]
fn the_published_schema_is_the_generated_one() {
    let published =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../schemas/import-profile-v1.json");
    let generated = import_profile_json_schema();
    assert_eq!(
        IMPORT_PROFILE_SCHEMA_ID,
        "https://polandy.github.io/framehive/schemas/import-profile-v1.json"
    );
    assert!(generated.contains(&format!("\"$id\": \"{IMPORT_PROFILE_SCHEMA_ID}\"")));
    for key in ["destination", "naming", "job_codes", "backup"] {
        assert!(generated.contains(&format!("\n    \"{key}\": {{")), "{key}");
    }
    assert_eq!(
        fs::read_to_string(published).unwrap(),
        generated,
        "run `cargo xtask schemas`"
    );
}
