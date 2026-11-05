// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The YAML tree a [`crate::YamlParser`] builds: what a file says, with the position
//! of every node and the style of every scalar (ADR 0032, decision 2).

use std::fmt;

/// A place in the text of a file, counted from 1. The column counts characters, not
/// bytes, so `ü` is one column (CORE-006.19).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    /// The line, from 1.
    pub line: usize,
    /// The column in characters, from 1.
    pub column: usize,
}

impl Position {
    /// The first character of a file, where errors about the file as a whole go.
    pub const START: Position = Position { line: 1, column: 1 };

    /// The position at `line` and `column`, both counted from 1.
    #[must_use]
    pub const fn new(line: usize, column: usize) -> Self {
        Position { line, column }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.line, self.column)
    }
}

/// How a scalar was written. Only a plain scalar is resolved by the YAML core schema
/// (`1` is a whole number, `true` a boolean); every other style is text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarStyle {
    /// Written without quotes, such as `destination: /a`.
    Plain,
    /// Written in single or double quotes, or as a literal (`|`) or folded (`>`)
    /// block.
    Quoted,
}

/// One node of the YAML tree and where it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlNode {
    /// The first character of the node: of a scalar, of `[`, `{` or `-`, or of the
    /// first key of a block mapping.
    pub position: Position,
    /// What the node holds.
    pub value: YamlValue,
}

/// The content of a [`YamlNode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YamlValue {
    /// A scalar: its text after YAML's unquoting and folding, and its style. A key
    /// without a value is a plain scalar with empty text.
    Scalar {
        /// The text of the scalar.
        text: String,
        /// Whether it was written plain or quoted.
        style: ScalarStyle,
    },
    /// A block or flow sequence, entries in file order.
    Sequence(Vec<YamlNode>),
    /// A block or flow mapping, entries in file order, repeated keys included.
    Mapping(Vec<YamlEntry>),
}

/// One key and its value in a mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlEntry {
    /// The key.
    pub key: YamlNode,
    /// The value.
    pub value: YamlNode,
}

impl YamlNode {
    /// A plain scalar at `position`, for building trees in tests and adapters.
    #[must_use]
    pub fn plain(position: Position, text: &str) -> Self {
        Self::scalar(position, text, ScalarStyle::Plain)
    }

    /// A quoted scalar at `position`.
    #[must_use]
    pub fn quoted(position: Position, text: &str) -> Self {
        Self::scalar(position, text, ScalarStyle::Quoted)
    }

    fn scalar(position: Position, text: &str, style: ScalarStyle) -> Self {
        YamlNode {
            position,
            value: YamlValue::Scalar {
                text: text.to_owned(),
                style,
            },
        }
    }
}
