// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Hierarchical keywords in `dc:subject`, `lr:hierarchicalSubject` and
//! `mwg-kw:Keywords` (ADR 0040, section 2).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use std::collections::BTreeMap;

use metadata::xmp::namespaces::{DUBLIN_CORE, LIGHTROOM, MWG_KEYWORDS};
use metadata::xmp::{ArrayKind, XmpEditError, XmpName, XmpNode, XmpPacket, XmpValue};

fn levels(keyword: &str) -> Vec<String> {
    keyword.split('/').map(str::to_owned).collect()
}

fn add(packet: &mut XmpPacket, keywords: &[&str]) -> Result<(), XmpEditError> {
    let keywords: Vec<Vec<String>> = keywords.iter().map(|keyword| levels(keyword)).collect();
    let keywords: Vec<&[String]> = keywords.iter().map(Vec::as_slice).collect();
    packet.add_keywords(&keywords)
}

fn mwg(local: &str) -> XmpName {
    XmpName::new(MWG_KEYWORDS, local)
}

fn bag(items: Vec<XmpNode>) -> XmpNode {
    XmpNode::new(XmpValue::Array(ArrayKind::Bag, items))
}

fn texts(items: &[&str]) -> XmpNode {
    bag(items.iter().map(|item| XmpNode::text(item)).collect())
}

/// A keyword node; `applied` is the `mwg-kw:Applied` field, if any.
fn node(keyword: &str, applied: Option<&str>, children: Vec<XmpNode>) -> XmpNode {
    let mut fields = BTreeMap::from([(mwg("Keyword"), XmpNode::text(keyword))]);
    if let Some(applied) = applied {
        fields.insert(mwg("Applied"), XmpNode::text(applied));
    }
    if !children.is_empty() {
        fields.insert(mwg("Children"), bag(children));
    }
    XmpNode::new(XmpValue::Struct(fields))
}

fn tree(hierarchy: Vec<XmpNode>) -> XmpNode {
    XmpNode::new(XmpValue::Struct(BTreeMap::from([(
        mwg("Hierarchy"),
        bag(hierarchy),
    )])))
}

fn subject() -> XmpName {
    XmpName::new(DUBLIN_CORE, "subject")
}

fn hierarchical_subject() -> XmpName {
    XmpName::new(LIGHTROOM, "hierarchicalSubject")
}

/// spec: IMPORT-019.1
#[test]
fn a_hierarchical_keyword_is_written_as_leaf_path_and_tree() {
    let mut packet = XmpPacket::new();

    add(&mut packet, &["Holiday/Beach"]).unwrap();

    assert_eq!(packet.property(&subject()), Some(&texts(&["Beach"])));
    assert_eq!(
        packet.property(&hierarchical_subject()),
        Some(&texts(&["Holiday|Beach"]))
    );
    assert_eq!(
        packet.property(&mwg("Keywords")),
        Some(&tree(vec![node(
            "Holiday",
            None,
            vec![node("Beach", None, vec![])]
        )]))
    );
}

/// spec: IMPORT-019.1
#[test]
fn keywords_are_unioned_and_levels_under_the_same_parent_merged() {
    let mut packet = XmpPacket::new();
    add(&mut packet, &["Holiday/Sun", "Sea"]).unwrap();

    add(&mut packet, &["Holiday/Beach", "Sea", "Holiday/Sun"]).unwrap();

    assert_eq!(
        packet.property(&subject()),
        Some(&texts(&["Sun", "Sea", "Beach"]))
    );
    assert_eq!(
        packet.property(&hierarchical_subject()),
        Some(&texts(&["Holiday|Sun", "Sea", "Holiday|Beach"]))
    );
    assert_eq!(
        packet.property(&mwg("Keywords")),
        Some(&tree(vec![
            node(
                "Holiday",
                None,
                vec![node("Sun", None, vec![]), node("Beach", None, vec![])]
            ),
            node("Sea", None, vec![]),
        ]))
    );
}

#[test]
fn a_leaf_that_gains_a_child_stays_applied() {
    let mut packet = XmpPacket::new();
    add(&mut packet, &["Holiday"]).unwrap();

    add(&mut packet, &["Holiday/Beach"]).unwrap();

    assert_eq!(
        packet.property(&mwg("Keywords")),
        Some(&tree(vec![node(
            "Holiday",
            Some("True"),
            vec![node("Beach", None, vec![])]
        )]))
    );
}

#[test]
fn an_ancestor_added_as_keyword_becomes_applied() {
    let mut packet = XmpPacket::new();
    add(&mut packet, &["Holiday/Beach"]).unwrap();

    add(&mut packet, &["Holiday"]).unwrap();

    assert_eq!(
        packet.property(&mwg("Keywords")),
        Some(&tree(vec![node(
            "Holiday",
            Some("True"),
            vec![node("Beach", None, vec![])]
        )]))
    );
    assert_eq!(
        packet.property(&subject()),
        Some(&texts(&["Beach", "Holiday"]))
    );
}

#[test]
fn a_property_of_another_form_is_refused_and_nothing_changes() {
    let mut packet = XmpPacket::new();
    packet
        .replace_seq(&hierarchical_subject(), &["Holiday|Beach"])
        .unwrap();
    let before = packet.clone();

    let error = add(&mut packet, &["Holiday/Sun"]).unwrap_err();

    assert!(matches!(error, XmpEditError::WrongForm { .. }));
    assert_eq!(packet, before);
    assert_eq!(packet.property(&subject()), None);
}
