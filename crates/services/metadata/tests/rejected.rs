// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The rejected pick state as `xmp:Rating` below zero (ADR 0044).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use metadata::xmp::namespaces::XMP_BASIC;
use metadata::xmp::{XmpName, XmpPacket};

fn rated(rating: &str) -> XmpPacket {
    let mut packet = XmpPacket::new();
    packet
        .set_text(&XmpName::new(XMP_BASIC, "Rating"), rating)
        .unwrap();
    packet
}

#[test]
fn a_rating_below_zero_is_rejected() {
    for rating in ["-1", " -1 ", "-1.0"] {
        assert!(rated(rating).is_rejected(), "{rating:?}");
    }
}

#[test]
fn stars_text_and_no_rating_are_not_rejected() {
    for rating in ["0", "1", "5", "", "rejected", "NaN"] {
        assert!(!rated(rating).is_rejected(), "{rating:?}");
    }
    assert!(!XmpPacket::new().is_rejected());
}

#[test]
fn a_rating_of_another_form_is_not_rejected() {
    let mut packet = XmpPacket::new();
    packet
        .replace_seq(&XmpName::new(XMP_BASIC, "Rating"), &["-1"])
        .unwrap();
    assert!(!packet.is_rejected());
}
