// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sidecars that other programs wrote survive a Framehive edit (CORE-010.9).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod common;

use common::{dc, parse, write_and_read};
use metadata::xmp::namespaces::LIGHTROOM;
use metadata::xmp::{XmpName, XmpNode, XmpValue};

/// darktable sidecars of the project owner's shots, one raw or DNG and one JPEG per camera.
const DARKTABLE: [&str; 8] = [
    "darktable/eos-350d-cosmos.dng.xmp",
    "darktable/eos-350d-cosmos.JPG.xmp",
    "darktable/eos-7d-garden-house.dng.xmp",
    "darktable/eos-7d-garden-house.JPG.xmp",
    "darktable/x-t2-stone-villa.RAF.xmp",
    "darktable/x-t2-stone-villa.JPG.xmp",
    "darktable/x-t5-mountain-lake.raf.xmp",
    "darktable/x-t5-mountain-lake.jpg.xmp",
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

        let keyword_properties = [
            dc("subject"),
            XmpName::new(LIGHTROOM, "hierarchicalSubject"),
        ];
        for (property, value) in original.properties() {
            let found = read_back.properties().get(property);
            if keyword_properties.contains(property) {
                assert!(keeps_every_item(value, found), "{name}: {property:?}");
            } else {
                assert_eq!(found, Some(value), "{name}: {property:?}");
            }
        }
        for (property, merged) in keyword_properties.iter().zip(["Lisbon", "Holiday|Lisbon"]) {
            let merged = XmpNode::new(XmpValue::Text(merged.to_owned()));
            let items = array_items(read_back.properties().get(property));
            assert!(items.contains(&merged), "{name}: {property:?}");
        }
    }
}

/// Whether the array `found` still holds every item of the array `original`.
fn keeps_every_item(original: &XmpNode, found: Option<&XmpNode>) -> bool {
    let found = array_items(found);
    array_items(Some(original))
        .iter()
        .all(|item| found.contains(item))
}

/// The items of an array node, or none for a missing or non-array node.
fn array_items(node: Option<&XmpNode>) -> &[XmpNode] {
    match node.map(|node| &node.value) {
        Some(XmpValue::Array(_, items)) => items,
        _ => &[],
    }
}
