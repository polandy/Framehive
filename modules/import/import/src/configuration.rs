// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `config.yaml` in the configuration folder, the file type `configuration`
//! (IMPORT-019, ADR 0040 §2). For now it holds only the `metadata` the import writes,
//! so the import module owns its description.

use config::{ConfigError, FileType, YamlParser};

use crate::metadata_values::{METADATA_KEY, MetadataKeys};

/// The name of the file in the configuration folder.
pub const CONFIGURATION_FILE_NAME: &str = "config.yaml";

/// The `$id` of the published JSON Schema, `schemas/configuration-v1.json`.
pub const CONFIGURATION_SCHEMA_ID: &str =
    "https://polandy.github.io/framehive/schemas/configuration-v1.json";

/// The file type `configuration`, version 1.
pub const CONFIGURATION: FileType = FileType::new("configuration", 1, &[METADATA_KEY]);

/// The JSON Schema of [`CONFIGURATION`], as published in
/// `schemas/configuration-v1.json`.
#[must_use]
pub fn configuration_json_schema() -> String {
    config::json_schema(&CONFIGURATION, CONFIGURATION_SCHEMA_ID)
}

/// The settings of `config.yaml`; a missing file sets nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Configuration {
    /// The `metadata` keys the file sets.
    pub metadata: MetadataKeys,
}

impl Configuration {
    /// Reads `config.yaml` named `file` from `text`. Reading is pure: `file` is used
    /// only in messages.
    ///
    /// # Errors
    ///
    /// The first error of CORE-006, else the first invalid keyword, at its value.
    pub fn read(parser: &dyn YamlParser, file: &str, text: &str) -> Result<Self, ConfigError> {
        let config = config::read(parser, &CONFIGURATION, file, text)?;
        Ok(Configuration {
            metadata: MetadataKeys::from_config(&config)?,
        })
    }
}
