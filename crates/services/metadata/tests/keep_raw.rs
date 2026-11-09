// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Keep RAW marker as `framehive:KeepRaw` (LIBRARY-002, ADR 0042).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use metadata::xmp::namespaces::{FRAMEHIVE, XMP_BASIC};
use metadata::xmp::{XmpEditError, XmpName, XmpPacket};

fn keep_raw() -> XmpName {
    XmpName::new(FRAMEHIVE, "KeepRaw")
}

fn rated(rating: &str) -> XmpPacket {
    let mut packet = XmpPacket::new();
    packet
        .set_text(&XmpName::new(XMP_BASIC, "Rating"), rating)
        .unwrap();
    packet
}

/// spec: LIBRARY-002.3
#[test]
fn marking_writes_true_and_keeps_the_other_properties() {
    let mut packet = rated("4");

    packet.set_keep_raw(true).unwrap();

    let mut expected = rated("4");
    expected.set_text(&keep_raw(), "True").unwrap();
    assert_eq!(packet, expected);
    assert!(packet.keep_raw());
}

/// spec: LIBRARY-002.4
#[test]
fn unmarking_removes_the_property() {
    let mut packet = rated("4");
    packet.set_keep_raw(true).unwrap();

    packet.set_keep_raw(false).unwrap();

    assert_eq!(packet, rated("4"));
    assert!(!packet.keep_raw());
}

/// spec: LIBRARY-002.4
#[test]
fn unmarking_an_unmarked_packet_changes_nothing() {
    let mut packet = rated("2");

    packet.set_keep_raw(false).unwrap();

    assert_eq!(packet, rated("2"));
}

/// spec: LIBRARY-002.5
#[test]
fn only_true_in_any_case_marks_the_asset() {
    for (value, marked) in [
        ("True", true),
        ("true", true),
        ("TRUE", true),
        ("False", false),
        ("", false),
        ("yes", false),
    ] {
        let mut packet = XmpPacket::new();
        packet.set_text(&keep_raw(), value).unwrap();

        assert_eq!(packet.keep_raw(), marked, "value {value:?}");
    }
    assert!(!XmpPacket::new().keep_raw());
}

/// spec: LIBRARY-002.6
#[test]
fn a_value_of_another_form_is_neither_read_nor_overwritten() {
    let mut packet = XmpPacket::new();
    packet.add_to_bag(&keep_raw(), &["True"]).unwrap();
    let before = packet.clone();

    let marked = packet.set_keep_raw(true);
    let unmarked = packet.set_keep_raw(false);

    assert!(matches!(marked, Err(XmpEditError::WrongForm { .. })));
    assert!(matches!(unmarked, Err(XmpEditError::WrongForm { .. })));
    assert_eq!(packet, before);
    assert!(!packet.keep_raw());
}
