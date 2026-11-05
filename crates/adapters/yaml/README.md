<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-yaml

Layer: `adapter`, port `config` (architecture §3)

## Purpose

Implements `config::YamlParser` with the event parser of `saphyr-parser` (ADR 0032,
decision 1): YAML 1.2, with no tag interpreted. `saphyr`'s document loader is not
used; this crate builds the `YamlNode` tree from the events, so every key and value
keeps its position and every scalar its style. No other crate sees a `saphyr-parser`
type.

While it builds the tree, it rejects at the first character of the construct:

- an anchor or an alias;
- a tag, including the core ones such as `!!str`;
- a second document: `---` after content, or a second `---`.

A leading `---` and comments are allowed. A text without a document is `None`.
Merge keys (`<<`) and everything about keys and values are left to `config`.
Positions are 1-based; the column counts characters, as the parser does. Single and
double quoted, literal and folded scalars are all `ScalarStyle::Quoted`.

## Public API

| Item | Meaning |
|---|---|
| `SaphyrParser` | `config::YamlParser` with `saphyr-parser`; `SaphyrParser::new()` or `Default` |

## Conformance

`tests/conformance.rs` runs `config::testing::check_yaml_parser`. `tests/reading.rs`
tests the criteria of CORE-006 end to end, `config::read` with this parser.

## Dependencies

- `config`
- `saphyr-parser` (ADR 0032): version 0.3, no features
