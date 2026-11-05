// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The port that turns YAML text into a [`YamlNode`] tree (ADR 0032, decision 2).

use crate::tree::{Position, YamlNode};

/// Parses YAML 1.2 text into a tree, rejecting what ADR 0011, rule 5, forbids.
///
/// `adapter-yaml` implements it; [`crate::testing::check_yaml_parser`] is its
/// conformance suite.
pub trait YamlParser {
    /// Parses `text`. Returns `None` for a text without a document (empty, or only
    /// comments), and the root node otherwise.
    ///
    /// # Errors
    ///
    /// The first syntax error, anchor, alias, tag or second document in the text,
    /// at the position of its first character.
    fn parse(&self, text: &str) -> Result<Option<YamlNode>, YamlParseError>;
}

/// Why a text could not be parsed, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlParseError {
    /// The first character of the offending construct, or the parser's position of
    /// a syntax error.
    pub position: Position,
    /// What was wrong.
    pub kind: YamlParseErrorKind,
}

/// The kinds of [`YamlParseError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YamlParseErrorKind {
    /// The text is not valid YAML; the parser's description.
    Syntax(String),
    /// An anchor (`&name`) or alias (`*name`).
    AnchorOrAlias,
    /// A tag (`!name`, `!!str`).
    Tag,
    /// A second document: `---` after content, or a second `---`.
    SecondDocument,
}
