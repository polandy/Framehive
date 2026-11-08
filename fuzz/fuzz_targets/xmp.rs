// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Fuzz target `xmp`: any bytes go to the XMP parser through the `quick-xml` adapter
//! (ADR 0037, ADR 0039).
//!
//! A panic or an allocation above libFuzzer's limit is a finding, and so is a packet
//! that does not read back the same after it is written (CORE-009).

#![no_main]

use std::path::Path;

use adapter_xmp::QuickXml;
use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use libfuzzer_sys::fuzz_target;
use metadata::xmp::{XmpName, XmpNode, namespaces::XMP_BASIC, parse_xmp, serialize_xmp};

fuzz_target!(|content: &[u8]| {
    let xml = QuickXml::new();
    let Ok(packet) = parse_xmp(Path::new("fuzz.xmp"), content, &xml) else {
        return;
    };
    let clock = ManualClock::new(Timestamp::from_unix_seconds(0), UtcOffset::UTC);
    let written = serialize_xmp(&packet, &clock, &xml).expect("1970 is a valid XMP year");
    let read_back = parse_xmp(Path::new("written.xmp"), written.as_bytes(), &xml)
        .expect("a written packet reads back");
    let mut expected = packet.properties().clone();
    expected.insert(
        XmpName::new(XMP_BASIC, "MetadataDate"),
        XmpNode::text("1970-01-01T00:00:00+00:00"),
    );
    assert_eq!(read_back.properties(), &expected);
});
