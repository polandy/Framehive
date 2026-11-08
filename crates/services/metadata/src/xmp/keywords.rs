// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Hierarchical keywords in the three properties other programs read (ADR 0040,
//! section 2): the leaf in `dc:subject`, the levels joined by `|` in
//! `lr:hierarchicalSubject` and the tree in `mwg-kw:Keywords` (MWG 2.0, section 5.7).

use std::collections::BTreeMap;

use super::error::XmpEditError;
use super::model::{ArrayKind, XmpName, XmpNode, XmpPacket, XmpValue};
use super::namespaces::{DUBLIN_CORE, LIGHTROOM, MWG_KEYWORDS};

/// The separator of levels in `lr:hierarchicalSubject`.
const LEVEL_SEPARATOR: &str = "|";
/// The XMP Boolean true.
const TRUE: &str = "True";

impl XmpPacket {
    /// Adds each keyword, given as its levels from the root to the leaf, unless the
    /// packet already has it. In `mwg-kw:Keywords`, a level that exists under the same
    /// parent is reused. The packet stays unchanged when an error is returned.
    ///
    /// # Errors
    ///
    /// [`XmpEditError::WrongForm`] when one of the three properties, or a part of the
    /// keyword tree on the way to a keyword, has another form than MWG and XMP define.
    pub fn add_keywords(&mut self, keywords: &[&[String]]) -> Result<(), XmpEditError> {
        let keywords: Vec<&[String]> = keywords
            .iter()
            .copied()
            .filter(|levels| !levels.is_empty())
            .collect();
        let leaves: Vec<&str> = keywords
            .iter()
            .filter_map(|levels| levels.last())
            .map(String::as_str)
            .collect();
        let joined: Vec<String> = keywords
            .iter()
            .map(|levels| levels.join(LEVEL_SEPARATOR))
            .collect();
        let joined: Vec<&str> = joined.iter().map(String::as_str).collect();

        let mut edited = self.clone();
        edited.add_to_bag(&XmpName::new(DUBLIN_CORE, "subject"), &leaves)?;
        edited.add_to_bag(&XmpName::new(LIGHTROOM, "hierarchicalSubject"), &joined)?;
        let hierarchy = edited.keyword_hierarchy()?;
        for levels in &keywords {
            add_to_tree(hierarchy, levels)?;
        }
        *self = edited;
        Ok(())
    }

    /// The items of `mwg-kw:Keywords/mwg-kw:Hierarchy`, created empty when missing.
    fn keyword_hierarchy(&mut self) -> Result<&mut Vec<XmpNode>, XmpEditError> {
        let keywords = mwg("Keywords");
        let node = self
            .properties
            .entry(keywords.clone())
            .or_insert_with(|| XmpNode::new(XmpValue::Struct(BTreeMap::new())));
        match &mut node.value {
            XmpValue::Struct(fields) => bag_field(fields, &mwg("Hierarchy")),
            _ => Err(wrong_form(&keywords, "a struct")),
        }
    }
}

/// Adds the path `levels` below `siblings`, reusing nodes with the same keyword.
fn add_to_tree(mut siblings: &mut Vec<XmpNode>, levels: &[String]) -> Result<(), XmpEditError> {
    let Some((leaf, ancestors)) = levels.split_last() else {
        return Ok(());
    };
    for level in ancestors {
        let existed = siblings.iter().any(|node| keyword_of(node) == Some(level));
        let fields = child_named(siblings, level)?;
        // A leaf without `Applied` counts as applied, an ancestor as not (MWG 2.0).
        if existed && !has_children(fields) && !fields.contains_key(&mwg("Applied")) {
            fields.insert(mwg("Applied"), XmpNode::text(TRUE));
        }
        siblings = bag_field(fields, &mwg("Children"))?;
    }
    let fields = child_named(siblings, leaf)?;
    if has_children(fields) || fields.contains_key(&mwg("Applied")) {
        fields.insert(mwg("Applied"), XmpNode::text(TRUE));
    }
    Ok(())
}

/// The fields of the node with the keyword `keyword` among `siblings`, appended when
/// missing. A new node has the field `mwg-kw:Keyword` only.
fn child_named<'a>(
    siblings: &'a mut Vec<XmpNode>,
    keyword: &str,
) -> Result<&'a mut BTreeMap<XmpName, XmpNode>, XmpEditError> {
    let position = siblings
        .iter()
        .position(|node| keyword_of(node) == Some(keyword));
    let index = position.unwrap_or_else(|| {
        let fields = BTreeMap::from([(mwg("Keyword"), XmpNode::text(keyword))]);
        siblings.push(XmpNode::new(XmpValue::Struct(fields)));
        siblings.len() - 1
    });
    match &mut siblings[index].value {
        XmpValue::Struct(fields) => Ok(fields),
        _ => Err(wrong_form(&mwg("Keyword"), "a field of a struct")),
    }
}

/// The text of the field `mwg-kw:Keyword` of a struct node.
fn keyword_of(node: &XmpNode) -> Option<&str> {
    match &node.value {
        XmpValue::Struct(fields) => match &fields.get(&mwg("Keyword"))?.value {
            XmpValue::Text(text) => Some(text),
            _ => None,
        },
        _ => None,
    }
}

/// Whether the struct has a non-empty `mwg-kw:Children`.
fn has_children(fields: &BTreeMap<XmpName, XmpNode>) -> bool {
    matches!(
        fields.get(&mwg("Children")).map(|node| &node.value),
        Some(XmpValue::Array(_, items)) if !items.is_empty()
    )
}

/// The items of the unordered array in the field `name`, created empty when missing.
fn bag_field<'a>(
    fields: &'a mut BTreeMap<XmpName, XmpNode>,
    name: &XmpName,
) -> Result<&'a mut Vec<XmpNode>, XmpEditError> {
    let node = fields
        .entry(name.clone())
        .or_insert_with(|| XmpNode::new(XmpValue::Array(ArrayKind::Bag, Vec::new())));
    match &mut node.value {
        XmpValue::Array(ArrayKind::Bag, items) => Ok(items),
        _ => Err(wrong_form(name, "an unordered array")),
    }
}

fn mwg(local: &str) -> XmpName {
    XmpName::new(MWG_KEYWORDS, local)
}

fn wrong_form(name: &XmpName, expected: &'static str) -> XmpEditError {
    XmpEditError::WrongForm {
        property: name.to_string(),
        expected,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

    use super::*;

    #[test]
    fn unknown_fields_of_a_keyword_node_survive() {
        let unknown = XmpName::new("http://example.com/ns/", "note");
        let holiday = BTreeMap::from([
            (mwg("Keyword"), XmpNode::text("Holiday")),
            (unknown.clone(), XmpNode::text("kept")),
        ]);
        let hierarchy = vec![XmpNode::new(XmpValue::Struct(holiday))];
        let mut packet = XmpPacket::new();
        packet.keyword_hierarchy().unwrap().extend(hierarchy);

        packet
            .add_keywords(&[&["Holiday".to_owned(), "Beach".to_owned()]])
            .unwrap();

        let holiday = &packet.keyword_hierarchy().unwrap()[0];
        let XmpValue::Struct(fields) = &holiday.value else {
            unreachable!("a keyword node is a struct");
        };
        assert_eq!(fields.get(&unknown), Some(&XmpNode::text("kept")));
        assert!(has_children(fields));
    }
}
