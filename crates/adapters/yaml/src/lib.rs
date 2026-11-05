// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `yaml` adapter of Framehive: it implements the [`config::YamlParser`] port
//! with the event parser of `saphyr-parser` (ADR 0032, decision 1). Only apps depend
//! on it. See `README.md`.

#![forbid(unsafe_code)]

mod builder;

use config::{YamlNode, YamlParseError, YamlParseErrorKind, YamlParser};
use saphyr_parser::Parser;

use crate::builder::{TreeBuilder, position};

/// [`YamlParser`] with `saphyr-parser`: YAML 1.2, no tags interpreted, and the tree
/// built from the parser's events by our own code, so every node keeps its position
/// and every scalar its style.
#[derive(Debug, Clone, Copy, Default)]
pub struct SaphyrParser;

impl SaphyrParser {
    /// The parser; it holds no state between texts.
    #[must_use]
    pub fn new() -> Self {
        SaphyrParser
    }
}

impl YamlParser for SaphyrParser {
    fn parse(&self, text: &str) -> Result<Option<YamlNode>, YamlParseError> {
        let mut parser = Parser::new_from_str(text);
        let mut builder = TreeBuilder::new(text);
        while let Some(next) = parser.next_event() {
            let (event, span) = next.map_err(|error| YamlParseError {
                position: position(*error.marker()),
                kind: YamlParseErrorKind::Syntax(error.info().to_owned()),
            })?;
            builder.push(event, span)?;
        }
        Ok(builder.finish())
    }
}
