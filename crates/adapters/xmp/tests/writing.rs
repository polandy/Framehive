// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Merging into packets and writing them in the canonical form (CORE-009).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod common;

use std::collections::BTreeMap;

use adapter_xmp::QuickXml;
use common::{
    ACME, WRITE_TIME, dc, fixture, parse, parse_bytes, photoshop, serialize, write_and_read, xmp,
};
use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use metadata::xmp::{
    ArrayKind, ClockOutOfRange, XmpEditError, XmpName, XmpNode, XmpPacket, XmpValue, serialize_xmp,
};

const XMP_MEDIA_MANAGEMENT: &str = "http://ns.adobe.com/xap/1.0/mm/";

/// spec: CORE-009.3
#[test]
fn unknown_fields_survive_a_merge() {
    let original = parse("merge.xmp");
    let mut merged = original.clone();
    merged
        .set_text(&photoshop("TransmissionReference"), "Lisbon")
        .unwrap();

    let read_back = write_and_read(&merged);

    let mut expected: BTreeMap<_, _> = original.properties().clone();
    expected.insert(photoshop("TransmissionReference"), XmpNode::text("Lisbon"));
    expected.insert(xmp("MetadataDate"), XmpNode::text(WRITE_TIME));
    assert_eq!(read_back.properties(), &expected);
    let batch = original.property(&XmpName::new(ACME, "Batch")).unwrap();
    assert_eq!(batch, &XmpNode::text("  7 & 8  "));
}

/// spec: CORE-009.4
#[test]
fn the_canonical_form_matches_the_fixture() {
    let written = serialize(&parse("attributes.xmp"));
    let expected = String::from_utf8(fixture("canonical.xmp")).unwrap();
    assert_eq!(written, expected);
    assert!(!written.contains("xpacket"));
}

#[test]
fn the_canonical_form_reads_back_unchanged() {
    let canonical = parse("canonical.xmp");
    assert_eq!(
        serialize(&canonical),
        String::from_utf8(fixture("canonical.xmp")).unwrap()
    );
}

/// spec: CORE-009.5
#[test]
fn serializing_does_not_depend_on_the_order_of_changes() {
    let mut first = XmpPacket::new();
    first.set_text(&xmp("Rating"), "3").unwrap();
    first.add_to_bag(&dc("subject"), &["Sea"]).unwrap();
    first.set_language(&dc("title"), "de", "Hafen").unwrap();
    first
        .set_language(&dc("title"), "x-default", "Harbour")
        .unwrap();
    first.set_text(&XmpName::new(ACME, "Batch"), "7").unwrap();

    let mut second = XmpPacket::new();
    second.set_text(&XmpName::new(ACME, "Batch"), "7").unwrap();
    second
        .set_language(&dc("title"), "x-default", "Harbour")
        .unwrap();
    second.add_to_bag(&dc("subject"), &["Sea"]).unwrap();
    second.set_language(&dc("title"), "de", "Hafen").unwrap();
    second.set_text(&xmp("Rating"), "3").unwrap();

    assert_eq!(serialize(&first), serialize(&second));
}

/// spec: CORE-009.6
#[test]
fn known_namespaces_get_their_prefix_and_unknown_ones_keep_theirs() {
    let written = serialize(&parse("foreign-prefixes.xmp"));
    assert!(
        written.contains("<dc:format>image/x-fuji-raf</dc:format>"),
        "{written}"
    );
    assert!(written.contains("<acme:Batch>7</acme:Batch>"), "{written}");
    assert!(
        written.contains(&format!("xmlns:acme=\"{ACME}\"")),
        "{written}"
    );
    assert!(!written.contains("xmlns:d="), "{written}");
}

/// spec: CORE-009.7
#[test]
fn a_clashing_prefix_gets_a_number() {
    let original = parse("clashing-prefix.xmp");
    let written = serialize(&original);
    assert!(
        written.contains("<dc:format>image/jpeg</dc:format>"),
        "{written}"
    );
    assert!(
        written.contains("<dc1:format>custom</dc1:format>"),
        "{written}"
    );
    let read_back = parse_bytes("written.xmp", written.as_bytes()).unwrap();
    let mut expected = original.properties().clone();
    expected.insert(xmp("MetadataDate"), XmpNode::text(WRITE_TIME));
    assert_eq!(read_back.properties(), &expected);
}

/// spec: CORE-009.8
#[test]
fn unordered_arrays_are_unioned() {
    let mut packet = parse("subject.xmp");
    packet.add_to_bag(&dc("subject"), &["Sea", "Sun"]).unwrap();
    let items = ["Beach", "Sea", "Sun"].map(XmpNode::text).to_vec();
    assert_eq!(
        packet.property(&dc("subject")),
        Some(&XmpNode::new(XmpValue::Array(ArrayKind::Bag, items)))
    );
}

/// spec: CORE-009.9
#[test]
fn one_language_is_set() {
    let mut packet = parse("rights.xmp");
    packet
        .set_language(&dc("rights"), "x-default", "© 2026 Andy Pollari")
        .unwrap();
    let read_back = write_and_read(&packet);
    let Some(XmpValue::Array(ArrayKind::Alt, items)) =
        read_back.property(&dc("rights")).map(|node| &node.value)
    else {
        panic!("dc:rights is no language alternative");
    };
    let languages: Vec<_> = items
        .iter()
        .map(|item| (item.language().unwrap(), item.value.clone()))
        .collect();
    assert_eq!(
        languages,
        [
            (
                "x-default",
                XmpValue::Text("© 2026 Andy Pollari".to_owned())
            ),
            ("de", XmpValue::Text("© Andy".to_owned())),
        ]
    );
}

/// spec: CORE-009.9
#[test]
fn a_language_is_found_without_regard_to_case() {
    let mut packet = parse("rights.xmp");
    packet.set_language(&dc("rights"), "DE", "© A. P.").unwrap();
    let Some(XmpValue::Array(ArrayKind::Alt, items)) =
        packet.property(&dc("rights")).map(|node| &node.value)
    else {
        panic!("dc:rights is no language alternative");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[1].language(), Some("de"));
    assert_eq!(items[1].value, XmpValue::Text("© A. P.".to_owned()));
}

/// spec: CORE-009.10
#[test]
fn only_the_metadata_date_is_kept_up_to_date() {
    let read_back = write_and_read(&parse("darktable.xmp"));
    assert_eq!(
        read_back.property(&xmp("MetadataDate")),
        Some(&XmpNode::text(WRITE_TIME))
    );
    assert_eq!(
        read_back.property(&xmp("CreatorTool")),
        Some(&XmpNode::text("darktable"))
    );
    assert_eq!(read_back.property(&xmp("ModifyDate")), None);
    assert_eq!(
        read_back.property(&XmpName::new(XMP_MEDIA_MANAGEMENT, "History")),
        None
    );
}

#[test]
fn an_array_is_replaced_but_never_flattened() {
    let mut packet = parse("merge.xmp");
    let tags = XmpName::new(ACME, "Tags");
    assert!(packet.set_text(&tags, "flat").is_err());
    packet.replace_seq(&tags, &["only"]).unwrap();
    assert_eq!(
        packet.property(&tags),
        Some(&XmpNode::new(XmpValue::Array(
            ArrayKind::Seq,
            vec![XmpNode::text("only")]
        )))
    );
}

#[test]
fn every_change_refuses_a_property_of_another_form() {
    let mut packet = parse("merge.xmp");
    packet.add_to_bag(&dc("subject"), &["Sea"]).unwrap();
    packet.set_language(&dc("rights"), "de", "© A.").unwrap();
    let refusals = [
        packet.add_to_bag(&dc("rights"), &["Sea"]),
        packet.replace_seq(&dc("subject"), &["Sea"]),
        packet.set_language(&dc("subject"), "de", "Meer"),
        packet.set_text(&dc("subject"), "Sea"),
    ];
    for refusal in refusals {
        assert!(matches!(refusal, Err(XmpEditError::WrongForm { .. })));
    }
}

#[test]
fn a_clock_before_the_year_one_is_refused() {
    let clock = ManualClock::new(
        Timestamp::from_unix_seconds(-62_200_000_000),
        UtcOffset::from_seconds(0).unwrap(),
    );
    let written = serialize_xmp(&parse("subject.xmp"), &clock, &QuickXml::new());
    assert_eq!(written, Err(ClockOutOfRange));
}
