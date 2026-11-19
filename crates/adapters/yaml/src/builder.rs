// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Builds the [`YamlNode`] tree from the events of `saphyr-parser`, rejecting what
//! ADR 0011, rule 5, forbids as soon as it appears.

use std::borrow::Cow;

use config::{
    Position, ScalarStyle, YamlEntry, YamlNode, YamlParseError, YamlParseErrorKind, YamlValue,
};
use saphyr_parser::{Event, Marker, ScalarStyle as SaphyrStyle, Span, Tag};

/// The character that starts an anchor, as opposed to a tag (`!`).
const ANCHOR_INDICATOR: char = '&';

/// The tree under construction: the collections not yet closed, innermost last.
pub(crate) struct TreeBuilder<'t> {
    text: &'t str,
    open: Vec<Open>,
    root: Option<YamlNode>,
    documents: usize,
}

/// A collection whose end event has not come yet.
enum Open {
    Sequence {
        position: Position,
        entries: Vec<YamlNode>,
    },
    Mapping {
        position: Position,
        entries: Vec<YamlEntry>,
        key: Option<YamlNode>,
    },
}

/// The position of a `saphyr-parser` marker in `text`, counted from its character index
/// because the parser's line and column can be wrong after unusual input. An index past
/// the end, as after wide characters in a directive, is put after the last character.
/// Like YAML, lines break at `\n`, `\r` and `\r\n` (CORE-006.22).
pub(crate) fn position(text: &str, marker: Marker) -> Position {
    let mut line = 1;
    let mut column = 1;
    let mut previous = None;
    for character in text.chars().take(marker.index()) {
        match character {
            '\n' if previous == Some('\r') => {}
            '\n' | '\r' => {
                line += 1;
                column = 1;
            }
            _ => column += 1,
        }
        previous = Some(character);
    }
    Position::new(line, column)
}

impl<'t> TreeBuilder<'t> {
    /// A builder for the events of `text`, which it reads to tell an anchor from a
    /// tag at the same position.
    pub(crate) fn new(text: &'t str) -> Self {
        TreeBuilder {
            text,
            open: Vec::new(),
            root: None,
            documents: 0,
        }
    }

    /// Adds one event.
    pub(crate) fn push(&mut self, event: Event<'_>, span: Span) -> Result<(), YamlParseError> {
        let start = span.start;
        match event {
            Event::Nothing | Event::StreamStart | Event::StreamEnd | Event::DocumentEnd => Ok(()),
            Event::DocumentStart(_) => self.start_document(start),
            Event::Alias(_) => Err(error(self.text, start, YamlParseErrorKind::AnchorOrAlias)),
            Event::Scalar(text, style, anchor, tag) => {
                self.reject_properties(start, anchor, tag.as_ref())?;
                self.add(YamlNode {
                    position: position(self.text, start),
                    value: YamlValue::Scalar {
                        text: text.into_owned(),
                        style: scalar_style(style),
                    },
                });
                Ok(())
            }
            Event::SequenceStart(anchor, tag) => {
                self.reject_properties(start, anchor, tag.as_ref())?;
                self.open.push(Open::Sequence {
                    position: position(self.text, start),
                    entries: Vec::new(),
                });
                Ok(())
            }
            Event::MappingStart(anchor, tag) => {
                self.reject_properties(start, anchor, tag.as_ref())?;
                self.open.push(Open::Mapping {
                    position: position(self.text, start),
                    entries: Vec::new(),
                    key: None,
                });
                Ok(())
            }
            Event::SequenceEnd | Event::MappingEnd => {
                self.close();
                Ok(())
            }
        }
    }

    /// The root node, or `None` when the text has no document.
    pub(crate) fn finish(self) -> Option<YamlNode> {
        self.root
    }

    fn start_document(&mut self, start: Marker) -> Result<(), YamlParseError> {
        self.documents += 1;
        if self.documents > 1 {
            return Err(error(self.text, start, YamlParseErrorKind::SecondDocument));
        }
        Ok(())
    }

    /// Rejects an anchor or a tag on a node. The node's span starts at whichever was
    /// written first, so the character there tells which to report.
    fn reject_properties(
        &self,
        start: Marker,
        anchor: usize,
        tag: Option<&Cow<'_, Tag>>,
    ) -> Result<(), YamlParseError> {
        let has_anchor = anchor > 0;
        let has_tag = tag.is_some();
        if !has_anchor && !has_tag {
            return Ok(());
        }
        let anchor_first = has_anchor
            && (!has_tag || self.text.chars().nth(start.index()) == Some(ANCHOR_INDICATOR));
        let kind = if anchor_first {
            YamlParseErrorKind::AnchorOrAlias
        } else {
            YamlParseErrorKind::Tag
        };
        Err(error(self.text, start, kind))
    }

    fn close(&mut self) {
        let node = match self.open.pop() {
            Some(Open::Sequence { position, entries }) => YamlNode {
                position,
                value: YamlValue::Sequence(entries),
            },
            Some(Open::Mapping {
                position, entries, ..
            }) => YamlNode {
                position,
                value: YamlValue::Mapping(entries),
            },
            // The parser balances start and end events.
            None => return,
        };
        self.add(node);
    }

    /// Adds a finished node to the innermost open collection, or makes it the root.
    fn add(&mut self, node: YamlNode) {
        match self.open.last_mut() {
            None => self.root = Some(node),
            Some(Open::Sequence { entries, .. }) => entries.push(node),
            Some(Open::Mapping { entries, key, .. }) => match key.take() {
                None => *key = Some(node),
                Some(key) => entries.push(YamlEntry { key, value: node }),
            },
        }
    }
}

fn scalar_style(style: SaphyrStyle) -> ScalarStyle {
    match style {
        SaphyrStyle::Plain => ScalarStyle::Plain,
        SaphyrStyle::SingleQuoted
        | SaphyrStyle::DoubleQuoted
        | SaphyrStyle::Literal
        | SaphyrStyle::Folded => ScalarStyle::Quoted,
    }
}

fn error(text: &str, start: Marker, kind: YamlParseErrorKind) -> YamlParseError {
    YamlParseError {
        position: position(text, start),
        kind,
    }
}
