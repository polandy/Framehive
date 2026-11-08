// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading sidecars: every RDF and wrapper form, and the refused ones (CORE-009).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod common;

use std::collections::BTreeMap;

use common::{dc, fixture, parse, parse_bytes, xmp};
use metadata::xmp::{ArrayKind, XmpErrorReason, XmpName, XmpNode, XmpPacket, XmpValue};

const IPTC_CORE: &str = "http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/";

/// The model every CORE-009.1 fixture states.
fn expected_model() -> BTreeMap<XmpName, XmpNode> {
    let mut title = XmpNode::text("Lisbon harbour");
    title.qualifiers.insert(
        XmpName::new(metadata::xmp::namespaces::XML, "lang"),
        XmpNode::text("x-default"),
    );
    let contact = BTreeMap::from([
        (
            XmpName::new(IPTC_CORE, "CiAdrCity"),
            XmpNode::text("Lisbon"),
        ),
        (
            XmpName::new(IPTC_CORE, "CiEmailWork"),
            XmpNode::text("photos@example.org"),
        ),
    ]);
    BTreeMap::from([
        (
            dc("title"),
            XmpNode::new(XmpValue::Array(ArrayKind::Alt, vec![title])),
        ),
        (
            dc("subject"),
            XmpNode::new(XmpValue::Array(
                ArrayKind::Bag,
                vec![XmpNode::text("Beach"), XmpNode::text("Sea")],
            )),
        ),
        (xmp("Rating"), XmpNode::text("4")),
        (
            XmpName::new(IPTC_CORE, "CreatorContactInfo"),
            XmpNode::new(XmpValue::Struct(contact)),
        ),
    ])
}

/// spec: CORE-009.1
#[test]
fn every_rdf_form_gives_the_same_model() {
    let expected = expected_model();
    for name in [
        "attributes.xmp",
        "elements.xmp",
        "nested.xmp",
        "parse-type.xmp",
        "split.xmp",
    ] {
        assert_eq!(parse(name).properties(), &expected, "{name}");
    }
}

/// spec: CORE-009.2
#[test]
fn every_wrapper_and_root_form_gives_the_same_model() {
    let mut bare = "\u{feff}".as_bytes().to_vec();
    bare.extend(fixture("bare-rdf.xmp"));
    let bare = parse_bytes("bare-rdf.xmp", &bare).unwrap();
    let wrapped = parse("wrapped.xmp");
    let plain = parse("parse-type.xmp");
    assert_eq!(wrapped, plain);
    assert_eq!(bare, plain);
    assert_eq!(plain.properties(), &expected_model());
}

/// spec: CORE-009.11
#[test]
fn an_unclosed_description_is_refused_with_file_and_position() {
    let error = parse_bytes("unclosed.xmp", &fixture("unclosed.xmp")).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("unclosed.xmp"), "{message}");
    assert!(
        matches!(error.reason, XmpErrorReason::Malformed { line: 5, .. }),
        "{message}"
    );
}

/// spec: CORE-009.12
#[test]
fn a_document_type_declaration_is_refused_without_expanding_it() {
    let error = parse_bytes("document-type.xmp", &fixture("document-type.xmp")).unwrap_err();
    let message = error.to_string();
    assert_eq!(error.reason, XmpErrorReason::DocumentType);
    assert!(message.contains("document-type.xmp"), "{message}");
    assert!(message.contains("document type declaration"), "{message}");
    assert!(!message.contains("root:"), "{message}");
}

/// spec: CORE-009.13
#[test]
fn utf_16_is_refused_as_not_utf_8() {
    let text = String::from_utf8(fixture("subject.xmp")).unwrap();
    for (name, bytes) in [
        ("little-endian.xmp", utf16(&text, u16::to_le_bytes)),
        ("big-endian.xmp", utf16(&text, u16::to_be_bytes)),
    ] {
        let error = parse_bytes(name, &bytes).unwrap_err();
        assert_eq!(error.reason, XmpErrorReason::NotUtf8, "{name}");
        let message = error.to_string();
        assert!(
            message.contains(name) && message.contains("not UTF-8"),
            "{message}"
        );
    }
}

/// `text` in UTF-16 with a byte order mark.
fn utf16(text: &str, bytes: fn(u16) -> [u8; 2]) -> Vec<u8> {
    std::iter::once(0xFEFF)
        .chain(text.encode_utf16())
        .flat_map(bytes)
        .collect()
}

/// spec: CORE-009.14
#[test]
fn conflicting_values_are_refused_naming_the_property() {
    let error = parse_bytes("conflict.xmp", &fixture("conflict.xmp")).unwrap_err();
    let message = error.to_string();
    assert!(matches!(error.reason, XmpErrorReason::Conflict { .. }));
    assert!(message.contains("conflict.xmp"), "{message}");
    assert!(message.contains("Rating"), "{message}");
}

#[test]
fn equal_values_in_two_descriptions_are_one_property() {
    let text = String::from_utf8(fixture("conflict.xmp"))
        .unwrap()
        .replace("\"2\"", "\"4\"");
    let packet: XmpPacket = parse_bytes("same.xmp", text.as_bytes()).unwrap();
    assert_eq!(packet.property(&xmp("Rating")), Some(&XmpNode::text("4")));
}
