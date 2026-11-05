// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The strict reading of a YAML tree into the values of a file type (CORE-006).

use crate::core_schema::{self, CoreType};
use crate::description::{FileType, Key, KeyKind};
use crate::error::{ConfigError, Kind, Problem};
use crate::parser::{YamlParseError, YamlParseErrorKind, YamlParser};
use crate::suggest;
use crate::tree::{Position, ScalarStyle, YamlEntry, YamlNode, YamlValue};
use crate::values::{ConfigFile, Located, Value, Values};

/// The key every configuration file has (ADR 0011, rule 7).
const SCHEMA_VERSION: &str = "schema_version";
/// The key YAML 1.1 uses to merge mappings, which ADR 0011, rule 5, forbids.
const MERGE_KEY: &str = "<<";

/// Reads the configuration file `file` of `file_type` from `text`, parsed by
/// `parser`. `file` is used only in messages.
///
/// # Errors
///
/// The first error of CORE-006: first a syntax error or a forbidden YAML construct,
/// then `schema_version`, then the keys and values in file order, then missing keys.
pub fn read(
    parser: &dyn YamlParser,
    file_type: &FileType,
    file: &str,
    text: &str,
) -> Result<ConfigFile, ConfigError> {
    let error = |position, problem| ConfigError {
        file: file.to_owned(),
        position,
        problem,
    };
    let root = parser
        .parse(text)
        .map_err(|parse_error| parse_problem(parse_error, file))?;
    let Some(YamlNode {
        position,
        value: YamlValue::Mapping(entries),
    }) = root
    else {
        return Err(error(Position::START, Problem::NotAMapping));
    };
    let reader = Reader { file };
    let schema_version = reader.schema_version(&entries, file_type)?;
    let values = reader.mapping(position, &entries, file_type.keys(), Level::Root)?;
    let Some(schema_version) = schema_version else {
        return Err(error(
            Position::START,
            Problem::MissingKey(SCHEMA_VERSION.to_owned()),
        ));
    };
    Ok(ConfigFile {
        file: file.to_owned(),
        schema_version,
        values,
    })
}

/// Whether a mapping is the top level, where `schema_version` is a known key too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    Root,
    Nested,
}

/// Reads one file; holds the file name for its errors.
struct Reader<'a> {
    file: &'a str,
}

impl Reader<'_> {
    fn error(&self, position: Position, problem: Problem) -> ConfigError {
        ConfigError {
            file: self.file.to_owned(),
            position,
            problem,
        }
    }

    /// Checks the first `schema_version` before the other keys, because the version
    /// decides which keys are known. Returns `None` when it is missing.
    fn schema_version(
        &self,
        entries: &[YamlEntry],
        file_type: &FileType,
    ) -> Result<Option<u32>, ConfigError> {
        let Some(entry) = entries
            .iter()
            .find(|entry| plain_or_quoted_text(&entry.key) == Some(SCHEMA_VERSION))
        else {
            return Ok(None);
        };
        let value = &entry.value;
        let (text, style) = match &value.value {
            YamlValue::Scalar { text, style } => (text.as_str(), *style),
            _ => return Err(self.wrong_kind(Kind::WholeNumber, value)),
        };
        if style == ScalarStyle::Plain {
            match core_schema::resolve(text) {
                CoreType::Null => {
                    return Err(self.error(entry.key.position, no_value(SCHEMA_VERSION)));
                }
                CoreType::Integer => return self.version_number(text, value.position, file_type),
                CoreType::Float => {
                    return Err(self.error(value.position, Problem::VersionNotPositive));
                }
                CoreType::Boolean(_) | CoreType::Text => {}
            }
        }
        Err(self.wrong_kind(Kind::WholeNumber, value))
    }

    fn version_number(
        &self,
        text: &str,
        position: Position,
        file_type: &FileType,
    ) -> Result<Option<u32>, ConfigError> {
        let too_new = || {
            self.error(
                position,
                Problem::VersionTooNew {
                    version: text.to_owned(),
                    highest: file_type.highest_version(),
                },
            )
        };
        let Some(version) = core_schema::integer_value(text) else {
            return Err(if text.starts_with('-') {
                self.error(position, Problem::VersionNotPositive)
            } else {
                too_new()
            });
        };
        if version < 1 {
            return Err(self.error(position, Problem::VersionNotPositive));
        }
        match u32::try_from(version) {
            Ok(version) if version <= file_type.highest_version() => Ok(Some(version)),
            _ => Err(too_new()),
        }
    }

    /// Reads the entries of one mapping in file order, then checks for missing
    /// required keys.
    fn mapping(
        &self,
        position: Position,
        entries: &[YamlEntry],
        keys: &'static [Key],
        level: Level,
    ) -> Result<Values, ConfigError> {
        let mut seen: Vec<&str> = Vec::new();
        let mut values = Vec::new();
        for entry in entries {
            let name = self.key_name(&entry.key)?;
            if seen.contains(&name) {
                return Err(self.error(entry.key.position, Problem::RepeatedKey(name.to_owned())));
            }
            seen.push(name);
            if level == Level::Root && name == SCHEMA_VERSION {
                continue;
            }
            let Some(key) = keys.iter().find(|key| key.name() == name) else {
                return Err(self.unknown_key(&entry.key, name, keys, level));
            };
            if is_null(&entry.value) {
                return Err(self.error(entry.key.position, no_value(name)));
            }
            values.push((key.name(), self.value(key, &entry.value)?));
        }
        if let Some(missing) = keys
            .iter()
            .find(|key| key.is_required() && !seen.contains(&key.name()))
        {
            return Err(self.error(position, Problem::MissingKey(missing.name().to_owned())));
        }
        Ok(Values {
            position,
            keys,
            entries: values,
        })
    }

    fn key_name<'n>(&self, key: &'n YamlNode) -> Result<&'n str, ConfigError> {
        match &key.value {
            YamlValue::Scalar {
                text,
                style: ScalarStyle::Plain,
            } if text == MERGE_KEY => Err(self.error(key.position, Problem::MergeKey)),
            YamlValue::Scalar { text, .. } => Ok(text),
            _ => Err(self.wrong_kind(Kind::Text, key)),
        }
    }

    fn unknown_key(
        &self,
        node: &YamlNode,
        name: &str,
        keys: &'static [Key],
        level: Level,
    ) -> ConfigError {
        let root_keys = match level {
            Level::Root => Some(SCHEMA_VERSION),
            Level::Nested => None,
        };
        let known = root_keys.into_iter().chain(keys.iter().map(Key::name));
        self.error(
            node.position,
            Problem::UnknownKey {
                key: name.to_owned(),
                suggestion: suggest::nearest(name, known).map(str::to_owned),
            },
        )
    }

    fn value(&self, key: &'static Key, node: &YamlNode) -> Result<Value, ConfigError> {
        match key.kind() {
            KeyKind::Text => self.text(node).map(Value::Text),
            KeyKind::WholeNumber => self.whole_number(node).map(Value::WholeNumber),
            KeyKind::Boolean => self.boolean(node).map(Value::Boolean),
            KeyKind::ListOfText => self.list_of_text(key.name(), node),
            KeyKind::Choice(choices) => self.choice(node, choices).map(Value::Text),
            KeyKind::Mapping(keys) => match &node.value {
                YamlValue::Mapping(entries) => self
                    .mapping(node.position, entries, keys, Level::Nested)
                    .map(Value::Mapping),
                _ => Err(self.wrong_kind(Kind::Mapping, node)),
            },
        }
    }

    fn text(&self, node: &YamlNode) -> Result<Located<String>, ConfigError> {
        let YamlValue::Scalar { text, style } = &node.value else {
            return Err(self.wrong_kind(Kind::Text, node));
        };
        if *style == ScalarStyle::Plain && reads_as_other_than_text(text) {
            return Err(self.error(node.position, Problem::QuoteText(text.clone())));
        }
        Ok(Located {
            value: text.clone(),
            position: node.position,
        })
    }

    fn choice(
        &self,
        node: &YamlNode,
        choices: &'static [&'static str],
    ) -> Result<Located<String>, ConfigError> {
        let text = self.text(node)?;
        if choices.contains(&text.value.as_str()) {
            return Ok(text);
        }
        Err(self.error(
            node.position,
            Problem::NotAChoice {
                choices,
                found: text.value,
            },
        ))
    }

    fn whole_number(&self, node: &YamlNode) -> Result<Located<i64>, ConfigError> {
        let Some(text) =
            plain_scalar(node).filter(|text| core_schema::resolve(text) == CoreType::Integer)
        else {
            return Err(self.wrong_kind(Kind::WholeNumber, node));
        };
        let Some(value) = core_schema::integer_value(text) else {
            return Err(self.error(node.position, Problem::NumberTooLarge(text.to_owned())));
        };
        Ok(Located {
            value,
            position: node.position,
        })
    }

    fn boolean(&self, node: &YamlNode) -> Result<Located<bool>, ConfigError> {
        match plain_scalar(node).map(core_schema::resolve) {
            Some(CoreType::Boolean(value)) => Ok(Located {
                value,
                position: node.position,
            }),
            _ => Err(self.wrong_kind(Kind::Boolean, node)),
        }
    }

    fn list_of_text(&self, name: &str, node: &YamlNode) -> Result<Value, ConfigError> {
        let YamlValue::Sequence(entries) = &node.value else {
            return Err(self.wrong_kind(Kind::List, node));
        };
        let mut texts = Vec::with_capacity(entries.len());
        for entry in entries {
            if is_null(entry) {
                return Err(self.error(entry.position, Problem::EmptyEntry(name.to_owned())));
            }
            texts.push(self.text(entry)?);
        }
        Ok(Value::ListOfText(node.position, texts))
    }

    fn wrong_kind(&self, expected: Kind, node: &YamlNode) -> ConfigError {
        self.error(
            node.position,
            Problem::WrongKind {
                expected,
                found: found_kind(node),
            },
        )
    }
}

fn parse_problem(error: YamlParseError, file: &str) -> ConfigError {
    let problem = match error.kind {
        YamlParseErrorKind::Syntax(description) => Problem::Syntax(description),
        YamlParseErrorKind::AnchorOrAlias => Problem::AnchorOrAlias,
        YamlParseErrorKind::Tag => Problem::Tag,
        YamlParseErrorKind::SecondDocument => Problem::SecondDocument,
    };
    ConfigError {
        file: file.to_owned(),
        position: error.position,
        problem,
    }
}

fn no_value(key: &str) -> Problem {
    Problem::NoValue(key.to_owned())
}

fn plain_scalar(node: &YamlNode) -> Option<&str> {
    match &node.value {
        YamlValue::Scalar {
            text,
            style: ScalarStyle::Plain,
        } => Some(text),
        _ => None,
    }
}

fn plain_or_quoted_text(node: &YamlNode) -> Option<&str> {
    match &node.value {
        YamlValue::Scalar { text, .. } => Some(text),
        _ => None,
    }
}

fn is_null(node: &YamlNode) -> bool {
    plain_scalar(node).is_some_and(|text| core_schema::resolve(text) == CoreType::Null)
}

/// Whether a plain scalar must be quoted to be text: the core schema reads it as a
/// number or a boolean, or it starts with a date (CORE-006, "Values").
fn reads_as_other_than_text(text: &str) -> bool {
    !matches!(core_schema::resolve(text), CoreType::Text) || core_schema::starts_with_date(text)
}

/// The kind a node holds, named as CORE-006 names what was found.
fn found_kind(node: &YamlNode) -> Kind {
    match &node.value {
        YamlValue::Sequence(_) => Kind::List,
        YamlValue::Mapping(_) => Kind::Mapping,
        YamlValue::Scalar {
            text,
            style: ScalarStyle::Plain,
        } => match core_schema::resolve(text) {
            CoreType::Integer => Kind::WholeNumber,
            CoreType::Boolean(_) => Kind::Boolean,
            CoreType::Null | CoreType::Float | CoreType::Text => Kind::Text,
        },
        YamlValue::Scalar { .. } => Kind::Text,
    }
}

#[cfg(test)]
mod tests;
