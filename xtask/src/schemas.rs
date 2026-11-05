// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask schemas` and `cargo xtask check-schemas`: the published JSON Schemas
//! of the configuration files, generated from their descriptions (ADR 0032,
//! decision 6).

use std::fs;
use std::path::Path;

/// One published schema: where it lives and how it is generated.
struct Schema {
    path: &'static str,
    generate: fn() -> String,
}

const SCHEMAS: [Schema; 1] = [Schema {
    path: "schemas/import-profile-v1.json",
    generate: import::import_profile_json_schema,
}];

/// Writes every schema below `root`.
pub(crate) fn write(root: &Path) -> Result<(), String> {
    for schema in &SCHEMAS {
        let path = root.join(schema.path);
        fs::write(&path, (schema.generate)())
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        println!("wrote {}", schema.path);
    }
    Ok(())
}

/// Every schema whose committed text differs from the generated one.
pub(crate) fn check(root: &Path) -> Vec<String> {
    SCHEMAS
        .iter()
        .filter_map(|schema| drift(root, schema))
        .collect()
}

fn drift(root: &Path, schema: &Schema) -> Option<String> {
    let committed = fs::read_to_string(root.join(schema.path)).ok();
    (committed.as_deref() != Some((schema.generate)().as_str())).then(|| {
        format!(
            "{} is missing or differs from the generated schema; run `cargo xtask schemas`",
            schema.path
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_schema_is_a_json_file_in_the_schemas_folder() {
        for schema in &SCHEMAS {
            assert!(schema.path.starts_with("schemas/"), "{}", schema.path);
            assert!(
                Path::new(schema.path)
                    .extension()
                    .is_some_and(|extension| extension == "json")
            );
        }
    }
}
