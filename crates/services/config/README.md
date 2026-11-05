<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# config

Layer: `service` (architecture §2, §3)

## Purpose

The config service holds what every hand-edited configuration file shares (CORE-006,
ADR 0011, ADR 0032): the YAML tree, the parser port, the strict reading of a tree
into the values of a file type, and the JSON Schema of a file type. The typed structs
of a file type live with its owner, such as the import profile in `import`.

Boundaries:

- **Pure.** It reads no files: the caller passes the text, the file's name for
  messages and a `YamlParser`. `adapter-yaml` implements the parser.
- **Strict.** Unknown keys (with "did you mean …?" at most 2 edits away), repeated
  keys, keys without a value, values of the wrong kind, plain scalars that YAML would
  read as a number, a boolean or a date where text is expected, and a missing,
  invalid or newer `schema_version` are errors. Anchors, aliases, tags and second
  documents are rejected by the parser, merge keys (`<<`) here.
- **Order.** The parser's errors come first, then `schema_version` (it decides which
  keys are known), then the keys and values in file order, then missing keys in each
  mapping. Reading stops at the first error.
- **Positions.** Every value keeps its line and column (in characters), so the owner
  of a file type can check a value further, such as a path or a template, and report
  the error at the value with `ConfigFile::error_at`, in the same
  `<file>:<line>:<column>: <message>` form.
- **Defaults.** Keys that are left out are absent from `Values`; the owner applies
  its defaults. `text_or_default` returns a documented text default.

## Public API

| Item | Meaning |
|---|---|
| `FileType::new(name, highest_version, keys)` | A file type; `schema_version` is implicit |
| `Key::text`, `whole_number`, `boolean`, `list_of_text`, `choice(name, description, choices)`, `mapping(name, description, keys)` | `const` key descriptions; `.required()`, `.with_default(DefaultValue)` |
| `KeyKind`, `DefaultValue` | What a key holds, and its documented default |
| `read(parser, file_type, file, text)` | Reads one file into `ConfigFile`, or the first `ConfigError` |
| `ConfigFile` | `schema_version()`, `values()`, `file()`, `error_at(position, message)` |
| `Values` | One mapping: `text`, `text_or_default`, `whole_number`, `boolean`, `text_list`, `mapping`, `position_of`, `position` |
| `Located<T>` | A value and its `Position` |
| `ConfigError`, `Problem`, `Kind` | The error, its message per CORE-006, and the kinds the messages name |
| `json_schema(file_type, id)` | The JSON Schema (draft 2020-12) with `$id`, the same text on every run |
| `YamlParser`, `YamlParseError`, `YamlParseErrorKind` | The parser port and its errors |
| `YamlNode`, `YamlValue`, `YamlEntry`, `ScalarStyle`, `Position` | The YAML tree |
| `testing::check_yaml_parser` | The conformance suite of `YamlParser` |

## Tests

Unit tests read hand-built trees, so they need no parser: values and positions,
wrong kinds, empty list entries, the `schema_version` rules, the core schema, the edit
distance and the JSON Schema (CORE-006.18). The criteria of CORE-006 are tested end
to end with the real parser in `adapter-yaml`, because a service must not depend on
an adapter (architecture §3).

## Dependencies

- `thiserror`
