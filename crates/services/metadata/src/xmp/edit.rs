// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The changes a merge makes to a packet (CORE-009, merging). Everything they do not
//! name stays exactly as read.

use std::collections::btree_map::Entry;

use super::error::XmpEditError;
use super::model::{ArrayKind, XmpName, XmpNode, XmpPacket, XmpValue};

const X_DEFAULT: &str = "x-default";

impl XmpPacket {
    /// Sets the simple value of `name`, keeping the qualifiers of an existing simple
    /// value.
    ///
    /// # Errors
    ///
    /// [`XmpEditError::WrongForm`] when `name` holds a struct or an array.
    pub fn set_text(&mut self, name: &XmpName, text: &str) -> Result<(), XmpEditError> {
        match self.properties.entry(name.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(XmpNode::text(text));
            }
            Entry::Occupied(mut entry) => match &mut entry.get_mut().value {
                value @ (XmpValue::Text(_) | XmpValue::Uri(_)) => {
                    *value = XmpValue::Text(text.to_owned());
                }
                _ => return Err(wrong_form(name, "a simple value")),
            },
        }
        Ok(())
    }

    /// Adds each of `items` to the unordered array `name` unless an item with that
    /// text is already in it; the array is created when missing.
    ///
    /// # Errors
    ///
    /// [`XmpEditError::WrongForm`] when `name` holds anything but an `rdf:Bag`.
    pub fn add_to_bag(&mut self, name: &XmpName, items: &[&str]) -> Result<(), XmpEditError> {
        let existing = self.array(name, ArrayKind::Bag, "an unordered array")?;
        for item in items {
            let present = existing
                .iter()
                .any(|node| node.value == XmpValue::Text((*item).to_owned()));
            if !present {
                existing.push(XmpNode::text(item));
            }
        }
        Ok(())
    }

    /// Replaces the ordered array `name` by `items`.
    ///
    /// # Errors
    ///
    /// [`XmpEditError::WrongForm`] when `name` holds anything but an `rdf:Seq`.
    pub fn replace_seq(&mut self, name: &XmpName, items: &[&str]) -> Result<(), XmpEditError> {
        let existing = self.array(name, ArrayKind::Seq, "an ordered array")?;
        *existing = items.iter().map(|item| XmpNode::text(item)).collect();
        Ok(())
    }

    /// Sets the text of one language in the language alternative `name`, leaving the
    /// other languages as they are. A new `x-default` goes first, as XMP asks; another
    /// new language goes last. Languages compare without regard to case (RFC 3066).
    ///
    /// # Errors
    ///
    /// [`XmpEditError::WrongForm`] when `name` holds anything but an `rdf:Alt`.
    pub fn set_language(
        &mut self,
        name: &XmpName,
        language: &str,
        text: &str,
    ) -> Result<(), XmpEditError> {
        let items = self.array(name, ArrayKind::Alt, "a language alternative")?;
        let same_language = |node: &&mut XmpNode| {
            node.language()
                .is_some_and(|item| item.eq_ignore_ascii_case(language))
        };
        if let Some(item) = items.iter_mut().find(same_language) {
            item.value = XmpValue::Text(text.to_owned());
            return Ok(());
        }
        let mut item = XmpNode::text(text);
        item.qualifiers
            .insert(XmpName::language(), XmpNode::text(language));
        if language.eq_ignore_ascii_case(X_DEFAULT) {
            items.insert(0, item);
        } else {
            items.push(item);
        }
        Ok(())
    }

    /// The items of the array `name` of `kind`, created empty when missing.
    fn array(
        &mut self,
        name: &XmpName,
        kind: ArrayKind,
        expected: &'static str,
    ) -> Result<&mut Vec<XmpNode>, XmpEditError> {
        let node = self
            .properties
            .entry(name.clone())
            .or_insert_with(|| XmpNode::new(XmpValue::Array(kind, Vec::new())));
        match &mut node.value {
            XmpValue::Array(found, items) if *found == kind => Ok(items),
            _ => Err(wrong_form(name, expected)),
        }
    }
}

pub(super) fn wrong_form(name: &XmpName, expected: &'static str) -> XmpEditError {
    XmpEditError::WrongForm {
        property: name.to_string(),
        expected,
    }
}
