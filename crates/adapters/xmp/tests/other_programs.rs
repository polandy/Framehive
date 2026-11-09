// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sidecars that other programs wrote survive a Framehive edit (CORE-010.9).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod common;

use common::{dc, parse, write_and_read};
use metadata::xmp::XmpName;

/// darktable sidecars of the project owner's Canon EOS 350D shot, a DNG and a JPEG.
const DARKTABLE: [&str; 2] = [
    "darktable/eos-350d-cosmos.dng.xmp",
    "darktable/eos-350d-cosmos.JPG.xmp",
];

const DARKTABLE_NAMESPACE: &str = "http://darktable.sf.net/";

/// spec: CORE-010.9
#[test]
fn darktable_sidecars_keep_every_property_after_a_keyword_is_merged() {
    for name in DARKTABLE {
        let original = parse(name);
        assert!(
            original
                .properties()
                .contains_key(&XmpName::new(DARKTABLE_NAMESPACE, "history")),
            "{name} has no darktable history"
        );
        let mut edited = original.clone();
        edited
            .add_keywords(&[&["Holiday".to_owned(), "Lisbon".to_owned()]])
            .unwrap();

        let read_back = write_and_read(&edited);

        for (property, value) in original.properties() {
            assert_eq!(
                read_back.properties().get(property),
                Some(value),
                "{name}: {property:?}"
            );
        }
        assert!(
            read_back.properties().contains_key(&dc("subject")),
            "{name}"
        );
    }
}
