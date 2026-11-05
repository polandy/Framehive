// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The error of reading a configuration file, in the form
//! `<file>:<line>:<column>: <message>` (CORE-006, "Errors").

use std::fmt;

use crate::tree::Position;

/// The first error in a configuration file: where it is and what was wrong.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{file}:{position}: {problem}")]
pub struct ConfigError {
    /// The file's name as the caller passed it.
    pub file: String,
    /// The first character of the offending key or value.
    pub position: Position,
    /// What was wrong.
    pub problem: Problem,
}

/// What was wrong in a configuration file; its `Display` is the message of CORE-006.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// The YAML parser's description of a syntax error.
    #[error("{0}")]
    Syntax(String),
    /// An anchor or an alias.
    #[error("anchors and aliases are not allowed")]
    AnchorOrAlias,
    /// A merge key `<<`.
    #[error("merge keys are not allowed")]
    MergeKey,
    /// A tag.
    #[error("tags are not allowed")]
    Tag,
    /// A second document.
    #[error("only one document is allowed")]
    SecondDocument,
    /// The top level is missing or is not a mapping.
    #[error("expected a mapping of keys")]
    NotAMapping,
    /// A key the file type does not know, and the nearest known key, if one is close.
    #[error("unknown key {key}{}", suggestion.as_ref().map(|known| format!("; did you mean {known}?")).unwrap_or_default())]
    UnknownKey {
        /// The key as written.
        key: String,
        /// A known key at most 2 edits away.
        suggestion: Option<String>,
    },
    /// The second occurrence of a key in one mapping.
    #[error("repeated key {0}")]
    RepeatedKey(String),
    /// A required key that is missing from its mapping.
    #[error("missing key {0}")]
    MissingKey(String),
    /// A key whose value is YAML null.
    #[error("{0} has no value; leave the key out to use the default")]
    NoValue(String),
    /// An entry of the list under the key that is YAML null.
    #[error("{0} has an entry with no value")]
    EmptyEntry(String),
    /// A value of the wrong kind.
    #[error("expected {expected}, found {found}")]
    WrongKind {
        /// What the file type expects.
        expected: Kind,
        /// What the file holds.
        found: Kind,
    },
    /// A plain scalar where text is expected that YAML would read as something else.
    #[error("write \"{0}\" in quotes to use it as text")]
    QuoteText(String),
    /// A whole number outside the range Framehive can hold.
    #[error("{0} is too large for a whole number")]
    NumberTooLarge(String),
    /// A `schema_version` that is not a positive whole number.
    #[error("schema_version must be a positive whole number")]
    VersionNotPositive,
    /// A `schema_version` above the highest version the file type knows.
    #[error(
        "schema_version {version} is newer than this Framehive supports ({highest}); update Framehive"
    )]
    VersionTooNew {
        /// The version as written.
        version: String,
        /// The highest version the file type knows.
        highest: u32,
    },
    /// A check of the file type's owner, such as an invalid path; its message.
    #[error("{0}")]
    Invalid(String),
}

/// The kinds of values of CORE-006, named as the messages name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Text.
    Text,
    /// A whole number.
    WholeNumber,
    /// True or false.
    Boolean,
    /// A list.
    List,
    /// A mapping.
    Mapping,
}

impl fmt::Display for Kind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Kind::Text => "text",
            Kind::WholeNumber => "a whole number",
            Kind::Boolean => "true or false",
            Kind::List => "a list",
            Kind::Mapping => "a mapping",
        })
    }
}
