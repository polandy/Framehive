// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Fixtures, names and the clock the XMP tests share.

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]
#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use std::path::{Path, PathBuf};

use adapter_xmp::QuickXml;
use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use metadata::xmp::namespaces::{DUBLIN_CORE, PHOTOSHOP, XMP_BASIC};
use metadata::xmp::{XmpError, XmpName, XmpPacket, parse_xmp, serialize_xmp};

/// The namespace of the unknown properties in the fixtures.
pub(crate) const ACME: &str = "http://example.com/ns/acme/1.0/";
/// `2026-11-08T14:03:12+01:00`, the time of every write.
pub(crate) const WRITE_TIME: &str = "2026-11-08T14:03:12+01:00";

/// The path of the fixture `name`.
pub(crate) fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// The bytes of the fixture `name`.
pub(crate) fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).unwrap()
}

/// Parses `content` as if read from `name`.
pub(crate) fn parse_bytes(name: &str, content: &[u8]) -> Result<XmpPacket, XmpError> {
    parse_xmp(Path::new(name), content, &QuickXml::new())
}

/// Parses the fixture `name`, which must be valid.
pub(crate) fn parse(name: &str) -> XmpPacket {
    parse_bytes(name, &fixture(name)).unwrap()
}

/// Serializes `packet` at [`WRITE_TIME`].
pub(crate) fn serialize(packet: &XmpPacket) -> String {
    serialize_xmp(packet, &clock(), &QuickXml::new()).unwrap()
}

/// Writes `packet` and reads the result back.
pub(crate) fn write_and_read(packet: &XmpPacket) -> XmpPacket {
    parse_bytes("written.xmp", serialize(packet).as_bytes()).unwrap()
}

/// A clock standing at [`WRITE_TIME`].
pub(crate) fn clock() -> ManualClock {
    ManualClock::new(
        Timestamp::from_unix_seconds(1_794_142_992),
        UtcOffset::from_seconds(3_600).unwrap(),
    )
}

/// A Dublin Core name.
pub(crate) fn dc(local: &str) -> XmpName {
    XmpName::new(DUBLIN_CORE, local)
}

/// An XMP basic name.
pub(crate) fn xmp(local: &str) -> XmpName {
    XmpName::new(XMP_BASIC, local)
}

/// A Photoshop name.
pub(crate) fn photoshop(local: &str) -> XmpName {
    XmpName::new(PHOTOSHOP, local)
}
