// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The JSON Schema (draft 2020-12) of a file type, generated from its description
//! (CORE-006, "JSON Schema"; ADR 0032, decision 6).
//!
//! The text is built in a fixed order with a fixed layout, so it is the same on every
//! run and can be compared with the committed schema.

use std::fmt::Write as _;

use crate::description::{DefaultValue, FileType, Key, KeyKind};

/// The meta-schema every generated schema declares.
const DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";
/// The indentation of one level.
const INDENT: &str = "  ";

/// The JSON Schema of `file_type`, with `$id` set to `id`, ending in a line break.
#[must_use]
pub fn json_schema(file_type: &FileType, id: &str) -> String {
    let version = Json::Number(i64::from(file_type.highest_version()));
    let schema_version = Json::Object(vec![
        (
            "description",
            Json::text(&format!(
                "Version of the format, {}",
                file_type.highest_version()
            )),
        ),
        ("type", Json::text("integer")),
        ("const", version),
    ]);
    let mut properties = vec![("schema_version", schema_version)];
    properties.extend(
        file_type
            .keys()
            .iter()
            .map(|key| (key.name(), property(key))),
    );
    let mut required = vec![Json::text("schema_version")];
    required.extend(required_names(file_type.keys()));
    let schema = Json::Object(vec![
        ("$schema", Json::text(DRAFT_2020_12)),
        ("$id", Json::text(id)),
        ("title", Json::text(file_type.name())),
        ("type", Json::text("object")),
        ("properties", Json::Object(properties)),
        ("required", Json::Array(required)),
        ("additionalProperties", Json::Boolean(false)),
    ]);
    let mut text = String::new();
    schema.write(&mut text, 0);
    text.push('\n');
    text
}

fn property(key: &Key) -> Json {
    let mut members = vec![("description", Json::text(key.description()))];
    match key.kind() {
        KeyKind::Text => members.push(("type", Json::text("string"))),
        KeyKind::WholeNumber => members.push(("type", Json::text("integer"))),
        KeyKind::Boolean => members.push(("type", Json::text("boolean"))),
        KeyKind::ListOfText => {
            members.push(("type", Json::text("array")));
            members.push(("items", Json::Object(vec![("type", Json::text("string"))])));
        }
        KeyKind::Mapping(keys) => {
            members.push(("type", Json::text("object")));
            let properties = keys.iter().map(|key| (key.name(), property(key))).collect();
            members.push(("properties", Json::Object(properties)));
            let required = required_names(keys);
            if !required.is_empty() {
                members.push(("required", Json::Array(required)));
            }
            members.push(("additionalProperties", Json::Boolean(false)));
        }
    }
    if let Some(default) = key.default() {
        members.push(("default", default_value(default)));
    }
    Json::Object(members)
}

fn required_names(keys: &[Key]) -> Vec<Json> {
    keys.iter()
        .filter(|key| key.is_required())
        .map(|key| Json::text(key.name()))
        .collect()
}

fn default_value(default: DefaultValue) -> Json {
    match default {
        DefaultValue::Text(text) => Json::text(text),
        DefaultValue::WholeNumber(number) => Json::Number(number),
        DefaultValue::Boolean(boolean) => Json::Boolean(boolean),
    }
}

/// The few JSON values a schema needs; objects keep their members in order.
enum Json {
    Text(String),
    Number(i64),
    Boolean(bool),
    Array(Vec<Json>),
    Object(Vec<(&'static str, Json)>),
}

impl Json {
    fn text(text: &str) -> Self {
        Json::Text(text.to_owned())
    }

    /// Writes the value at nesting `level`: one member or element per line, except
    /// for an array of scalars, which stays on one line.
    fn write(&self, out: &mut String, level: usize) {
        match self {
            Json::Text(text) => write_string(out, text),
            Json::Number(number) => {
                let _ = write!(out, "{number}");
            }
            Json::Boolean(boolean) => {
                let _ = write!(out, "{boolean}");
            }
            Json::Array(elements) => {
                out.push('[');
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        out.push_str(", ");
                    }
                    element.write(out, level);
                }
                out.push(']');
            }
            Json::Object(members) => {
                out.push('{');
                for (index, (name, value)) in members.iter().enumerate() {
                    out.push_str(if index > 0 { ",\n" } else { "\n" });
                    out.push_str(&INDENT.repeat(level + 1));
                    write_string(out, name);
                    out.push_str(": ");
                    value.write(out, level + 1);
                }
                if !members.is_empty() {
                    out.push('\n');
                    out.push_str(&INDENT.repeat(level));
                }
                out.push('}');
            }
        }
    }
}

/// Writes `text` as a JSON string (RFC 8259, §7).
fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if u32::from(control) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(control));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "https://example.org/schemas/test-v1.json";

    /// spec: CORE-006.18
    #[test]
    fn follows_the_description_and_is_the_same_on_every_run() {
        const FILE_TYPE: FileType = FileType::new(
            "test",
            1,
            &[Key::text("destination", "Folder to import into")],
        );
        let first = json_schema(&FILE_TYPE, ID);
        assert_eq!(first, json_schema(&FILE_TYPE, ID));
        let expected = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://example.org/schemas/test-v1.json",
  "title": "test",
  "type": "object",
  "properties": {
    "schema_version": {
      "description": "Version of the format, 1",
      "type": "integer",
      "const": 1
    },
    "destination": {
      "description": "Folder to import into",
      "type": "string"
    }
  },
  "required": ["schema_version"],
  "additionalProperties": false
}
"#;
        assert_eq!(first, expected);
    }

    #[test]
    fn describes_nested_mappings_lists_and_defaults() {
        const BACKUP: &[Key] = &[
            Key::list_of_text("destinations", "Backup destinations").required(),
            Key::whole_number("copies", "Copies").with_default(DefaultValue::WholeNumber(2)),
            Key::boolean("verify", "Verify \"all\"").with_default(DefaultValue::Boolean(true)),
        ];
        const FILE_TYPE: FileType = FileType::new(
            "test",
            3,
            &[
                Key::text("on_conflict", "Policy").with_default(DefaultValue::Text("add_suffix")),
                Key::mapping("backup", "Backup", BACKUP),
            ],
        );
        let schema = json_schema(&FILE_TYPE, ID);
        for expected in [
            r#""const": 3"#,
            r#""default": "add_suffix""#,
            "\"backup\": {\n      \"description\": \"Backup\",\n      \"type\": \"object\",",
            "\"items\": {\n            \"type\": \"string\"\n          }",
            r#""required": ["destinations"],"#,
            r#""default": 2"#,
            r#""description": "Verify \"all\"""#,
            r#""default": true"#,
        ] {
            assert!(schema.contains(expected), "{expected} missing in\n{schema}");
        }
        assert_eq!(
            schema.matches(r#""additionalProperties": false"#).count(),
            2
        );
    }

    #[test]
    fn escapes_control_characters() {
        let mut out = String::new();
        write_string(&mut out, "a\u{1}b\\");
        assert_eq!(out, r#""a\u0001b\\""#);
    }
}
