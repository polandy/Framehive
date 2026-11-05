// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`YamlParser`] port (CLAUDE.md, "Adding things").
//!
//! Every adapter runs [`check_yaml_parser`] in its `tests/conformance.rs`. It checks
//! the tree the port promises: positions in characters from 1, keys in file order,
//! scalar styles, and the constructs ADR 0011, rule 5, forbids.

use crate::parser::{YamlParseError, YamlParseErrorKind, YamlParser};
use crate::tree::{Position, ScalarStyle, YamlEntry, YamlNode, YamlValue};

/// Runs every check against `parser` and returns one message per failed check; an
/// empty list means the parser conforms.
#[must_use]
pub fn check_yaml_parser(parser: &dyn YamlParser) -> Vec<String> {
    let mut failures = Vec::new();
    for check in CHECKS {
        if let Err(failure) = (check.run)(parser) {
            failures.push(format!("{}: {failure}", check.name));
        }
    }
    failures
}

struct Check {
    name: &'static str,
    run: fn(&dyn YamlParser) -> Result<(), String>,
}

const CHECKS: &[Check] = &[
    Check {
        name: "a text without a document is None",
        run: no_document,
    },
    Check {
        name: "a block mapping keeps keys in order with positions",
        run: block_mapping,
    },
    Check {
        name: "flow collections start at their bracket",
        run: flow_collections,
    },
    Check {
        name: "quoted and block scalars are quoted",
        run: scalar_styles,
    },
    Check {
        name: "a key without a value is an empty plain scalar",
        run: empty_value,
    },
    Check {
        name: "columns count characters",
        run: character_columns,
    },
    Check {
        name: "repeated keys are kept",
        run: repeated_keys,
    },
    Check {
        name: "anchors and aliases are rejected at the anchor",
        run: anchors,
    },
    Check {
        name: "tags are rejected at the tag",
        run: tags,
    },
    Check {
        name: "only one document is allowed",
        run: documents,
    },
    Check {
        name: "syntax errors have a position",
        run: syntax_error,
    },
];

fn no_document(parser: &dyn YamlParser) -> Result<(), String> {
    for text in ["", "# only a comment\n", "\n\n"] {
        expect_equal(&format!("{text:?}"), &parse(parser, text)?, &None)?;
    }
    Ok(())
}

fn block_mapping(parser: &dyn YamlParser) -> Result<(), String> {
    let tree = parse(parser, "b: 1\na:\n  - x\n  - y\n")?;
    let expected = mapping(
        at(1, 1),
        vec![
            entry(plain(1, 1, "b"), plain(1, 4, "1")),
            entry(
                plain(2, 1, "a"),
                sequence(at(3, 3), vec![plain(3, 5, "x"), plain(4, 5, "y")]),
            ),
        ],
    );
    expect_equal("tree", &tree, &Some(expected))
}

fn flow_collections(parser: &dyn YamlParser) -> Result<(), String> {
    let tree = parse(parser, "a: [x, y]\nb: {c: d}\nc: {}\n")?;
    let expected = mapping(
        at(1, 1),
        vec![
            entry(
                plain(1, 1, "a"),
                sequence(at(1, 4), vec![plain(1, 5, "x"), plain(1, 8, "y")]),
            ),
            entry(
                plain(2, 1, "b"),
                mapping(at(2, 4), vec![entry(plain(2, 5, "c"), plain(2, 8, "d"))]),
            ),
            entry(plain(3, 1, "c"), mapping(at(3, 4), Vec::new())),
        ],
    );
    expect_equal("tree", &tree, &Some(expected))
}

fn scalar_styles(parser: &dyn YamlParser) -> Result<(), String> {
    let tree = parse(parser, "'a': \"1\"\nc: no\n")?;
    let expected = mapping(
        at(1, 1),
        vec![
            entry(quoted(1, 1, "a"), quoted(1, 6, "1")),
            entry(plain(2, 1, "c"), plain(2, 4, "no")),
        ],
    );
    expect_equal("tree", &tree, &Some(expected))?;
    // Only the style of a block scalar is promised, not where its position is.
    let values = mapping_values(parse(parser, "b: |\n  text\nc: >\n  more\n")?)?;
    let expected_values = [
        scalar_value("text\n", ScalarStyle::Quoted),
        scalar_value("more\n", ScalarStyle::Quoted),
    ];
    expect_equal("block scalars", &values, &expected_values.to_vec())
}

fn empty_value(parser: &dyn YamlParser) -> Result<(), String> {
    let values = mapping_values(parse(parser, "a:\nb: ~\n")?)?;
    let expected_values = [
        scalar_value("", ScalarStyle::Plain),
        scalar_value("~", ScalarStyle::Plain),
    ];
    expect_equal("values", &values, &expected_values.to_vec())
}

/// The values of a top-level mapping, without their positions.
fn mapping_values(tree: Option<YamlNode>) -> Result<Vec<YamlValue>, String> {
    match tree {
        Some(YamlNode {
            value: YamlValue::Mapping(entries),
            ..
        }) => Ok(entries.into_iter().map(|entry| entry.value.value).collect()),
        other => Err(format!("expected a mapping, got {other:?}")),
    }
}

fn character_columns(parser: &dyn YamlParser) -> Result<(), String> {
    let tree = parse(parser, "a: [\"Müller\", x]\n")?;
    let expected = mapping(
        at(1, 1),
        vec![entry(
            plain(1, 1, "a"),
            sequence(at(1, 4), vec![quoted(1, 5, "Müller"), plain(1, 15, "x")]),
        )],
    );
    expect_equal("tree", &tree, &Some(expected))
}

fn repeated_keys(parser: &dyn YamlParser) -> Result<(), String> {
    let tree = parse(parser, "a: 1\na: 2\n")?;
    let expected = mapping(
        at(1, 1),
        vec![
            entry(plain(1, 1, "a"), plain(1, 4, "1")),
            entry(plain(2, 1, "a"), plain(2, 4, "2")),
        ],
    );
    expect_equal("tree", &tree, &Some(expected))
}

fn anchors(parser: &dyn YamlParser) -> Result<(), String> {
    expect_error(
        parser,
        "a: &x /a\nb: *x\n",
        at(1, 4),
        &YamlParseErrorKind::AnchorOrAlias,
    )?;
    expect_error(
        parser,
        "a: &x\n  b: 1\n",
        at(1, 4),
        &YamlParseErrorKind::AnchorOrAlias,
    )?;
    expect_error(
        parser,
        "a: [&x 1]\n",
        at(1, 5),
        &YamlParseErrorKind::AnchorOrAlias,
    )
}

fn tags(parser: &dyn YamlParser) -> Result<(), String> {
    expect_error(parser, "a: !!str /a\n", at(1, 4), &YamlParseErrorKind::Tag)?;
    expect_error(
        parser,
        "a: !local [1]\n",
        at(1, 4),
        &YamlParseErrorKind::Tag,
    )?;
    expect_error(parser, "!!map\na: 1\n", at(1, 1), &YamlParseErrorKind::Tag)
}

fn documents(parser: &dyn YamlParser) -> Result<(), String> {
    expect_error(
        parser,
        "a: 1\n---\na: 2\n",
        at(2, 1),
        &YamlParseErrorKind::SecondDocument,
    )?;
    expect_error(
        parser,
        "---\n---\n",
        at(2, 1),
        &YamlParseErrorKind::SecondDocument,
    )?;
    let tree = parse(parser, "# comment\n---\na: 1\n")?;
    let expected = mapping(at(3, 1), vec![entry(plain(3, 1, "a"), plain(3, 4, "1"))]);
    expect_equal("tree after a leading ---", &tree, &Some(expected))
}

fn syntax_error(parser: &dyn YamlParser) -> Result<(), String> {
    match parser.parse("a: \"/a\n") {
        Err(YamlParseError {
            kind: YamlParseErrorKind::Syntax(description),
            position,
        }) if !description.is_empty() && position.line >= 1 && position.column >= 1 => Ok(()),
        other => Err(format!("expected a syntax error, got {other:?}")),
    }
}

fn parse(parser: &dyn YamlParser, text: &str) -> Result<Option<YamlNode>, String> {
    parser
        .parse(text)
        .map_err(|error| format!("{text:?} failed: {error:?}"))
}

fn expect_error(
    parser: &dyn YamlParser,
    text: &str,
    position: Position,
    kind: &YamlParseErrorKind,
) -> Result<(), String> {
    let expected = Err(YamlParseError {
        position,
        kind: kind.clone(),
    });
    expect_equal(&format!("{text:?}"), &parser.parse(text), &expected)
}

fn expect_equal<T: PartialEq + std::fmt::Debug>(
    what: &str,
    found: &T,
    expected: &T,
) -> Result<(), String> {
    if found == expected {
        Ok(())
    } else {
        Err(format!("{what}: expected {expected:?}, found {found:?}"))
    }
}

fn at(line: usize, column: usize) -> Position {
    Position::new(line, column)
}

fn plain(line: usize, column: usize, text: &str) -> YamlNode {
    YamlNode::plain(at(line, column), text)
}

fn quoted(line: usize, column: usize, text: &str) -> YamlNode {
    YamlNode::quoted(at(line, column), text)
}

fn scalar_value(text: &str, style: ScalarStyle) -> YamlValue {
    YamlValue::Scalar {
        text: text.to_owned(),
        style,
    }
}

fn sequence(position: Position, entries: Vec<YamlNode>) -> YamlNode {
    YamlNode {
        position,
        value: YamlValue::Sequence(entries),
    }
}

fn mapping(position: Position, entries: Vec<YamlEntry>) -> YamlNode {
    YamlNode {
        position,
        value: YamlValue::Mapping(entries),
    }
}

fn entry(key: YamlNode, value: YamlNode) -> YamlEntry {
    YamlEntry { key, value }
}
