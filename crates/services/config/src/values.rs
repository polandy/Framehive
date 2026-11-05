// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The values of a configuration file after the strict reading, each with its
//! position (CORE-006).

use crate::description::{DefaultValue, Key};
use crate::error::{ConfigError, Problem};
use crate::tree::Position;

/// A configuration file that passed the checks of its file type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFile {
    pub(crate) file: String,
    pub(crate) schema_version: u32,
    pub(crate) values: Values,
}

/// The keys present in one mapping of a file, each checked against the description.
///
/// Keys that are left out are absent here; the owner of the file type applies their
/// defaults. Asking for a key with an accessor of another kind returns `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Values {
    pub(crate) position: Position,
    pub(crate) keys: &'static [Key],
    pub(crate) entries: Vec<(&'static str, Value)>,
}

/// A value and the position of its first character, so a later check can report an
/// error at the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Located<T> {
    /// The value.
    pub value: T,
    /// Where the value starts.
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    Text(Located<String>),
    WholeNumber(Located<i64>),
    Boolean(Located<bool>),
    ListOfText(Position, Vec<Located<String>>),
    Mapping(Values),
}

impl ConfigFile {
    /// The file's name as passed to [`crate::read`].
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }

    /// The file's `schema_version`.
    #[must_use]
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// The top-level keys besides `schema_version`.
    #[must_use]
    pub fn values(&self) -> &Values {
        &self.values
    }

    /// An error at `position` in this file with the file type's own `message`, for a
    /// check the description cannot express, such as a path that is not absolute.
    #[must_use]
    pub fn error_at(&self, position: Position, message: impl Into<String>) -> ConfigError {
        ConfigError {
            file: self.file.clone(),
            position,
            problem: Problem::Invalid(message.into()),
        }
    }
}

impl Values {
    /// Where the mapping starts: its first key, or its `{`.
    #[must_use]
    pub fn position(&self) -> Position {
        self.position
    }

    /// The text under `key`, if present.
    #[must_use]
    pub fn text(&self, key: &str) -> Option<&Located<String>> {
        match self.get(key)? {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The text under `key`, or the default of its description when the key is
    /// absent and the default is text.
    #[must_use]
    pub fn text_or_default(&self, key: &str) -> Option<&str> {
        if let Some(text) = self.text(key) {
            return Some(&text.value);
        }
        match self.description(key)?.default()? {
            DefaultValue::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The whole number under `key`, if present.
    #[must_use]
    pub fn whole_number(&self, key: &str) -> Option<Located<i64>> {
        match self.get(key)? {
            Value::WholeNumber(number) => Some(*number),
            _ => None,
        }
    }

    /// The `true` or `false` under `key`, if present.
    #[must_use]
    pub fn boolean(&self, key: &str) -> Option<Located<bool>> {
        match self.get(key)? {
            Value::Boolean(boolean) => Some(*boolean),
            _ => None,
        }
    }

    /// The list of text under `key`, in file order, if present.
    #[must_use]
    pub fn text_list(&self, key: &str) -> Option<&[Located<String>]> {
        match self.get(key)? {
            Value::ListOfText(_, entries) => Some(entries),
            _ => None,
        }
    }

    /// The nested mapping under `key`, if present.
    #[must_use]
    pub fn mapping(&self, key: &str) -> Option<&Values> {
        match self.get(key)? {
            Value::Mapping(values) => Some(values),
            _ => None,
        }
    }

    /// Where the value under `key` starts, of any kind, if present.
    #[must_use]
    pub fn position_of(&self, key: &str) -> Option<Position> {
        Some(match self.get(key)? {
            Value::Text(text) => text.position,
            Value::WholeNumber(number) => number.position,
            Value::Boolean(boolean) => boolean.position,
            Value::ListOfText(position, _) => *position,
            Value::Mapping(values) => values.position,
        })
    }

    fn get(&self, key: &str) -> Option<&Value> {
        self.entries
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value)
    }

    fn description(&self, key: &str) -> Option<&'static Key> {
        self.keys.iter().find(|known| known.name() == key)
    }
}
