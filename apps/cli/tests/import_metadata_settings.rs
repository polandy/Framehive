// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `metadata` keys of `config.yaml` and of the import profile (IMPORT-019,
//! "Values") read with the real YAML parser, which the import module may not depend
//! on (architecture §3).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use std::path::Path;

use adapter_yaml::SaphyrParser;
use import::{Configuration, ImportProfile, Keyword, MetadataKeys};
use metadata::xmp::SidecarNaming;

const HOME: &str = "/home/ada";

fn profile(text: &str) -> Result<ImportProfile, String> {
    ImportProfile::read(&SaphyrParser::new(), "holiday.yaml", text, Path::new(HOME))
        .map_err(|error| error.to_string())
}

fn configuration(text: &str) -> Result<Configuration, String> {
    Configuration::read(&SaphyrParser::new(), "config.yaml", text)
        .map_err(|error| error.to_string())
}

fn keywords(texts: &[&str]) -> Vec<Keyword> {
    texts
        .iter()
        .map(|text| Keyword::parse(text).unwrap())
        .collect()
}

#[test]
fn config_yaml_sets_every_metadata_key() {
    let read = configuration(
        "schema_version: 1\nmetadata:\n  creator: [Andy Pollari]\n  copyright: © {year} Andy Pollari\n  keywords: [Holiday/Beach]\n  sidecar_naming: original_without_extension\n",
    )
    .unwrap();
    assert_eq!(
        read.metadata,
        MetadataKeys {
            creator: Some(vec!["Andy Pollari".to_owned()]),
            copyright: Some("© {year} Andy Pollari".to_owned()),
            keywords: Some(keywords(&["Holiday/Beach"])),
            sidecar_naming: Some(SidecarNaming::OriginalWithoutExtension),
        }
    );
    assert_eq!(
        configuration("schema_version: 1\nmetadata:\n  author: Ada\n").unwrap_err(),
        "config.yaml:3:3: unknown key author"
    );
}

/// spec: IMPORT-019.3
#[test]
fn the_profile_overrides_config_yaml_key_by_key() {
    let config = configuration(
        "schema_version: 1\nmetadata:\n  creator: [Andy Pollari]\n  keywords: [Family]\n",
    )
    .unwrap();
    let profile = profile("schema_version: 2\nmetadata:\n  keywords: [Holiday]\n").unwrap();
    let values = profile.metadata.over(&config.metadata).values();
    assert_eq!(values.creator, ["Andy Pollari"]);
    assert_eq!(values.keywords, keywords(&["Holiday"]));
}

/// spec: IMPORT-019.4
#[test]
fn a_version_1_profile_is_still_read() {
    let config =
        configuration("schema_version: 1\nmetadata:\n  creator: [Andy Pollari]\n").unwrap();
    let profile = profile("schema_version: 1\njob_codes: [Lisbon]\n").unwrap();
    assert_eq!(profile.metadata, MetadataKeys::default());
    assert_eq!(
        profile.metadata.over(&config.metadata).values().creator,
        ["Andy Pollari"]
    );
    assert_eq!(
        self::profile("schema_version: 1\nmetadata:\n  keywords: [Holiday]\n").unwrap_err(),
        "holiday.yaml:3:3: metadata needs schema_version 2"
    );
}

/// spec: IMPORT-019.5
#[test]
fn an_invalid_keyword_is_an_error_at_its_position() {
    assert_eq!(
        profile("schema_version: 2\nmetadata:\n  keywords:\n    - Family\n    - Holiday//Beach\n")
            .unwrap_err(),
        "holiday.yaml:5:7: a keyword must not start or end with / or contain //"
    );
    assert_eq!(
        configuration("schema_version: 1\nmetadata:\n  keywords: [\"Holiday|Beach\"]\n")
            .unwrap_err(),
        "config.yaml:3:14: a keyword must not contain |"
    );
}
