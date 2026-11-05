// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading hand-built trees, so these tests need no parser. The criteria of CORE-006
//! are tested end to end with the real parser in `adapter-yaml`.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use super::*;
use crate::DefaultValue;

const BACKUP: &[Key] = &[Key::list_of_text("destinations", "Backup destinations").required()];
const KEYS: &[Key] = &[
    Key::text("destination", "Folder to import into").with_default(DefaultValue::Text("/d")),
    Key::whole_number("copies", "Copies"),
    Key::boolean("verify", "Verify"),
    Key::mapping("backup", "Backup", BACKUP),
];
const FILE_TYPE: FileType = FileType::new("test", 1, KEYS);

/// A parser that returns a fixed tree.
struct Fixed(Option<YamlNode>);

impl YamlParser for Fixed {
    fn parse(&self, _text: &str) -> Result<Option<YamlNode>, YamlParseError> {
        Ok(self.0.clone())
    }
}

fn at(line: usize, column: usize) -> Position {
    Position::new(line, column)
}

fn entry(key: YamlNode, value: YamlNode) -> YamlEntry {
    YamlEntry { key, value }
}

/// A root mapping with `schema_version: 1` on line 1 and `rest` after it.
fn root(rest: Vec<YamlEntry>) -> YamlNode {
    let mut entries = vec![entry(
        YamlNode::plain(at(1, 1), "schema_version"),
        YamlNode::plain(at(1, 17), "1"),
    )];
    entries.extend(rest);
    YamlNode {
        position: at(1, 1),
        value: YamlValue::Mapping(entries),
    }
}

fn read_tree(tree: YamlNode) -> Result<ConfigFile, ConfigError> {
    read(&Fixed(Some(tree)), &FILE_TYPE, "f.yaml", "")
}

fn problem(tree: YamlNode) -> (Position, String) {
    let error = read_tree(tree).unwrap_err();
    (error.position, error.problem.to_string())
}

#[test]
fn gives_values_with_their_positions() {
    let file = read_tree(root(vec![
        entry(
            YamlNode::quoted(at(2, 1), "copies"),
            YamlNode::plain(at(2, 10), "0x10"),
        ),
        entry(
            YamlNode::plain(at(3, 1), "verify"),
            YamlNode::plain(at(3, 9), "False"),
        ),
        entry(
            YamlNode::plain(at(4, 1), "backup"),
            YamlNode {
                position: at(5, 3),
                value: YamlValue::Mapping(vec![entry(
                    YamlNode::plain(at(5, 3), "destinations"),
                    YamlNode {
                        position: at(5, 17),
                        value: YamlValue::Sequence(vec![YamlNode::plain(at(5, 18), "/b")]),
                    },
                )]),
            },
        ),
    ]))
    .unwrap();
    let values = file.values();
    assert_eq!(file.schema_version(), 1);
    assert_eq!(
        values.whole_number("copies"),
        Some(Located {
            value: 16,
            position: at(2, 10)
        })
    );
    assert_eq!(values.boolean("verify").map(|b| b.value), Some(false));
    assert_eq!(values.text_or_default("destination"), Some("/d"));
    assert_eq!(values.text("copies"), None);
    let backup = values.mapping("backup").unwrap();
    assert_eq!(backup.position(), at(5, 3));
    assert_eq!(backup.position_of("destinations"), Some(at(5, 17)));
    let destinations = backup.text_list("destinations").unwrap();
    assert_eq!(destinations[0].value, "/b");
    assert_eq!(destinations[0].position, at(5, 18));
}

#[test]
fn reports_an_owner_check_in_the_same_form() {
    let file = read_tree(root(Vec::new())).unwrap();
    let error = file.error_at(
        at(2, 14),
        "destination must be an absolute path or start with ~/",
    );
    assert_eq!(
        error.to_string(),
        "f.yaml:2:14: destination must be an absolute path or start with ~/"
    );
}

#[test]
fn rejects_a_key_that_is_not_text() {
    let key = YamlNode {
        position: at(2, 3),
        value: YamlValue::Sequence(Vec::new()),
    };
    let tree = root(vec![entry(key, YamlNode::plain(at(2, 8), "x"))]);
    assert_eq!(
        problem(tree),
        (at(2, 3), "expected text, found a list".to_owned())
    );
}

#[test]
fn rejects_an_empty_list_entry() {
    let list = YamlNode {
        position: at(3, 3),
        value: YamlValue::Sequence(vec![YamlNode::plain(at(3, 4), "~")]),
    };
    let backup = YamlNode {
        position: at(3, 3),
        value: YamlValue::Mapping(vec![entry(YamlNode::plain(at(3, 3), "destinations"), list)]),
    };
    let tree = root(vec![entry(YamlNode::plain(at(2, 1), "backup"), backup)]);
    assert_eq!(
        problem(tree),
        (
            at(3, 4),
            "destinations has an entry with no value".to_owned()
        )
    );
}

#[test]
fn rejects_wrong_kinds_of_numbers_and_booleans() {
    let tree = root(vec![entry(
        YamlNode::plain(at(2, 1), "copies"),
        YamlNode::quoted(at(2, 9), "2"),
    )]);
    assert_eq!(
        problem(tree),
        (at(2, 9), "expected a whole number, found text".to_owned())
    );
    let tree = root(vec![entry(
        YamlNode::plain(at(2, 1), "verify"),
        YamlNode::plain(at(2, 9), "yes"),
    )]);
    assert_eq!(
        problem(tree),
        (at(2, 9), "expected true or false, found text".to_owned())
    );
    let tree = root(vec![entry(
        YamlNode::plain(at(2, 1), "copies"),
        YamlNode::plain(at(2, 9), "99999999999999999999"),
    )]);
    assert_eq!(
        problem(tree),
        (
            at(2, 9),
            "99999999999999999999 is too large for a whole number".to_owned()
        )
    );
}

#[test]
fn rejects_schema_versions_that_are_not_positive_whole_numbers() {
    for (text, message) in [
        ("1.5", "schema_version must be a positive whole number"),
        ("-1", "schema_version must be a positive whole number"),
        (
            "-99999999999999999999",
            "schema_version must be a positive whole number",
        ),
        ("one", "expected a whole number, found text"),
        (
            "99999999999999999999",
            "schema_version 99999999999999999999 is newer than this Framehive supports (1); update Framehive",
        ),
    ] {
        let tree = YamlNode {
            position: at(1, 1),
            value: YamlValue::Mapping(vec![entry(
                YamlNode::plain(at(1, 1), "schema_version"),
                YamlNode::plain(at(1, 17), text),
            )]),
        };
        assert_eq!(problem(tree), (at(1, 17), message.to_owned()), "{text}");
    }
}

#[test]
fn checks_schema_version_before_other_keys() {
    let tree = YamlNode {
        position: at(1, 1),
        value: YamlValue::Mapping(vec![
            entry(
                YamlNode::plain(at(1, 1), "colour"),
                YamlNode::plain(at(1, 9), "red"),
            ),
            entry(
                YamlNode::plain(at(2, 1), "schema_version"),
                YamlNode::plain(at(2, 17), "2"),
            ),
        ]),
    };
    assert_eq!(problem(tree).0, at(2, 17));
}

#[test]
fn passes_on_the_parser_error() {
    struct Failing;
    impl YamlParser for Failing {
        fn parse(&self, _text: &str) -> Result<Option<YamlNode>, YamlParseError> {
            Err(YamlParseError {
                position: at(4, 2),
                kind: YamlParseErrorKind::Tag,
            })
        }
    }
    let error = read(&Failing, &FILE_TYPE, "f.yaml", "").unwrap_err();
    assert_eq!(error.to_string(), "f.yaml:4:2: tags are not allowed");
}
