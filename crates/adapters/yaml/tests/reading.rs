// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The criteria of CORE-006 end to end: `config::read` with the real parser. They
//! live here because `config` must not depend on an adapter (architecture §3).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use adapter_yaml::SaphyrParser;
use config::{ConfigError, ConfigFile, DefaultValue, FileType, Key, Position, read};

const BACKUP: &[Key] = &[Key::list_of_text("destinations", "Backup destinations").required()];
const KEYS: &[Key] = &[
    Key::text("destination", "Folder to import into").with_default(DefaultValue::Text("/default")),
    Key::list_of_text("job_codes", "Job codes"),
    Key::mapping("backup", "Backup copies", BACKUP),
];
const FILE_TYPE: FileType = FileType::new("test", 1, KEYS);
const FILE: &str = "holiday.yaml";

fn read_text(text: &str) -> Result<ConfigFile, ConfigError> {
    read(&SaphyrParser::new(), &FILE_TYPE, FILE, text)
}

fn error(text: &str) -> ConfigError {
    read_text(text).unwrap_err()
}

/// The error as `<line>:<column>: <message>`, without the file name.
fn located_message(text: &str) -> String {
    let error = error(text);
    format!("{}: {}", error.position, error.problem)
}

/// spec: CORE-006.1
#[test]
fn a_minimal_file_takes_the_defaults() {
    let file = read_text("schema_version: 1\n").unwrap();
    assert_eq!(file.schema_version(), 1);
    assert_eq!(file.values().text("destination"), None);
    assert_eq!(
        file.values().text_or_default("destination"),
        Some("/default")
    );
}

/// spec: CORE-006.2
#[test]
fn an_unknown_key_suggests_a_near_one() {
    assert_eq!(
        error("schema_version: 1\ndestinaton: /a\n").to_string(),
        "holiday.yaml:2:1: unknown key destinaton; did you mean destination?"
    );
}

/// spec: CORE-006.3
#[test]
fn an_unknown_key_far_from_all_has_no_suggestion() {
    assert_eq!(
        located_message("schema_version: 1\ncolour: red\n"),
        "2:1: unknown key colour"
    );
}

/// spec: CORE-006.4
#[test]
fn a_repeated_key_is_an_error_at_the_second() {
    assert_eq!(
        located_message("schema_version: 1\ndestination: /a\ndestination: /b\n"),
        "3:1: repeated key destination"
    );
}

/// spec: CORE-006.5
#[test]
fn anchors_and_aliases_are_rejected_at_the_anchor() {
    assert_eq!(
        located_message(
            "schema_version: 1\ndestination: &target /a\nbackup:\n  destinations: [*target]\n"
        ),
        "2:14: anchors and aliases are not allowed"
    );
}

/// spec: CORE-006.5
#[test]
fn an_alias_without_an_anchor_is_rejected_at_the_alias() {
    assert_eq!(
        located_message("schema_version: 1\ndestination: [*target]\n"),
        "2:15: anchors and aliases are not allowed"
    );
}

/// spec: CORE-006.6
#[test]
fn merge_keys_are_rejected_at_the_key() {
    assert_eq!(
        located_message("schema_version: 1\nbackup:\n  <<: {destinations: [/a]}\n"),
        "3:3: merge keys are not allowed"
    );
}

/// spec: CORE-006.7
#[test]
fn tags_are_rejected_at_the_tag() {
    assert_eq!(
        located_message("schema_version: 1\ndestination: !!str /a\n"),
        "2:14: tags are not allowed"
    );
}

/// spec: CORE-006.8
#[test]
fn only_one_document_is_allowed() {
    assert_eq!(
        located_message("schema_version: 1\n---\nschema_version: 1\n"),
        "2:1: only one document is allowed"
    );
    assert_eq!(
        read_text("---\nschema_version: 1\n")
            .unwrap()
            .schema_version(),
        1
    );
}

/// spec: CORE-006.9
#[test]
fn no_is_text() {
    let file = read_text("schema_version: 1\ndestination: no\n").unwrap();
    let destination = file.values().text("destination").unwrap();
    assert_eq!(destination.value, "no");
    assert_eq!(destination.position, Position::new(2, 14));
}

/// spec: CORE-006.10
#[test]
fn text_that_looks_like_a_number_or_a_date_must_be_quoted() {
    for (value, message) in [
        ("2026", "2:13: write \"2026\" in quotes to use it as text"),
        ("true", "2:13: write \"true\" in quotes to use it as text"),
        (
            "2026-11-05 Smith",
            "2:13: write \"2026-11-05 Smith\" in quotes to use it as text",
        ),
        ("1.5", "2:13: write \"1.5\" in quotes to use it as text"),
    ] {
        let text = format!("schema_version: 1\njob_codes: [{value}]\n");
        assert_eq!(located_message(&text), message, "{value}");
    }
    let file = read_text("schema_version: 1\njob_codes: [\"2026\"]\n").unwrap();
    let job_codes = file.values().text_list("job_codes").unwrap();
    assert_eq!(job_codes.len(), 1);
    assert_eq!(job_codes[0].value, "2026");
}

/// spec: CORE-006.11
#[test]
fn a_value_of_the_wrong_kind_names_what_was_expected() {
    assert_eq!(
        located_message("schema_version: 1\ndestination: [/a, /b]\n"),
        "2:14: expected text, found a list"
    );
    assert_eq!(
        located_message("schema_version: 1\njob_codes: Smith\n"),
        "2:12: expected a list, found text"
    );
    assert_eq!(
        located_message("schema_version: 1\nbackup: [/a]\n"),
        "2:9: expected a mapping, found a list"
    );
}

/// spec: CORE-006.12
#[test]
fn a_key_without_a_value_is_an_error_at_the_key() {
    let expected = "2:1: destination has no value; leave the key out to use the default";
    for text in [
        "schema_version: 1\ndestination:\n",
        "schema_version: 1\ndestination: ~\n",
        "schema_version: 1\ndestination: null\n",
    ] {
        assert_eq!(located_message(text), expected, "{text:?}");
    }
}

/// spec: CORE-006.13
#[test]
fn schema_version_must_be_present_and_a_positive_whole_number() {
    assert_eq!(
        located_message("destination: /a\n"),
        "1:1: missing key schema_version"
    );
    assert_eq!(
        located_message("schema_version: \"1\"\n"),
        "1:17: expected a whole number, found text"
    );
    assert_eq!(
        located_message("schema_version: 0\n"),
        "1:17: schema_version must be a positive whole number"
    );
    for text in [
        "schema_version:\n",
        "schema_version: ~\n",
        "schema_version: null\n",
    ] {
        assert_eq!(
            located_message(text),
            "1:1: schema_version has no value; it must be a positive whole number",
            "{text:?}"
        );
    }
    assert_eq!(
        located_message("destination: /a\nschema_version:\n"),
        "2:1: schema_version has no value; it must be a positive whole number"
    );
}

/// spec: CORE-006.14
#[test]
fn a_newer_schema_version_asks_for_an_update() {
    assert_eq!(
        located_message("schema_version: 2\n"),
        "1:17: schema_version 2 is newer than this Framehive supports (1); update Framehive"
    );
}

/// spec: CORE-006.15
#[test]
fn the_top_level_must_be_a_mapping() {
    for text in ["", "# only a comment\n", "- a\n", "just text\n"] {
        assert_eq!(
            located_message(text),
            "1:1: expected a mapping of keys",
            "{text:?}"
        );
    }
}

/// spec: CORE-006.16
#[test]
fn a_syntax_error_has_the_parsers_position() {
    let error = error("schema_version: 1\ndestination: \"/a\n");
    let message = error.to_string();
    let expected_start = format!(
        "holiday.yaml:{}:{}: ",
        error.position.line, error.position.column
    );
    assert!(message.starts_with(&expected_start), "{message}");
    assert!(message.len() > expected_start.len(), "{message}");
    assert!(
        matches!(error.problem, config::Problem::Syntax(_)),
        "{message}"
    );
}

/// spec: CORE-006.17
#[test]
fn a_missing_required_key_is_an_error_at_its_mapping() {
    assert_eq!(
        located_message("schema_version: 1\nbackup: {}\n"),
        "2:9: missing key destinations"
    );
    assert_eq!(
        located_message("schema_version: 1\nbackup:\n  destination: [/a]\n"),
        "3:3: unknown key destination; did you mean destinations?"
    );
}

/// spec: CORE-006.19
#[test]
fn columns_count_characters() {
    assert_eq!(
        located_message("schema_version: 1\njob_codes: [\"Hochzeit Müller\", 2026]\n"),
        "2:32: write \"2026\" in quotes to use it as text"
    );
}

/// spec: CORE-006.21
#[test]
fn a_list_entry_without_a_value_is_an_error() {
    assert_eq!(
        located_message("schema_version: 1\njob_codes: [Smith, ~]\n"),
        "2:20: job_codes has an entry with no value"
    );
}

/// spec: CORE-006.20
#[test]
fn a_choice_accepts_only_its_texts() {
    const CHOICE: FileType = FileType::new(
        "test",
        1,
        &[Key::choice(
            "on_conflict",
            "Conflict policy",
            &["add_suffix", "skip", "fail"],
        )],
    );
    let read_choice = |text| read(&SaphyrParser::new(), &CHOICE, FILE, text);
    let file = read_choice("schema_version: 1\non_conflict: skip\n").unwrap();
    assert_eq!(file.values().text("on_conflict").unwrap().value, "skip");
    let error = read_choice("schema_version: 1\non_conflict: overwrite\n").unwrap_err();
    assert_eq!(
        format!("{}: {}", error.position, error.problem),
        "2:14: expected add_suffix, skip or fail, found overwrite"
    );
}
