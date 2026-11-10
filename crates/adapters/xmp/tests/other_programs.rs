// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sidecars that other programs wrote survive a Framehive edit (CORE-010.9, CORE-010.11).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod common;

use common::{dc, parse, write_and_read};
use metadata::xmp::namespaces::{LIGHTROOM, XMP_BASIC};
use metadata::xmp::{XmpName, XmpNode, XmpPacket, XmpValue};

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

const MWG_REGIONS: &str = "http://www.metadataworkinggroup.com/schemas/regions/";

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
        assert_keyword_merge_keeps_every_property(name, &original);
    }
}

/// digiKam sidecars of the owner's photos, cleaned of people, GPS and serial numbers.
const DIGIKAM: [&str; 7] = [
    "digikam/eos-7d-pick-label.JPG.xmp",
    "digikam/optio-selection.JPG.xmp",
    "digikam/powershot-faces.JPG.xmp",
    "digikam/samsung-faces.jpg.xmp",
    "digikam/x-t2-colour-label.RAF.xmp",
    "digikam/x-t2-pick-label.RAF.xmp",
    "digikam/x-t2-tags.RAF.xmp",
];

const DIGIKAM_NAMESPACE: &str = "http://www.digikam.org/ns/1.0/";

/// spec: CORE-010.11
#[test]
fn digikam_sidecars_keep_every_property_after_a_keyword_is_merged() {
    for name in DIGIKAM {
        let original = parse(name);
        assert!(
            original
                .properties()
                .keys()
                .any(|property| property.namespace == DIGIKAM_NAMESPACE),
            "{name} has no digiKam property"
        );
        assert_keyword_merge_keeps_every_property(name, &original);
    }
}

/// spec: CORE-010.11
#[test]
fn digikam_sidecars_carry_labels_and_face_regions() {
    let label = |name: &str, property: &str| {
        let packet = parse(name);
        let node = packet
            .properties()
            .get(&XmpName::new(DIGIKAM_NAMESPACE, property))
            .unwrap_or_else(|| panic!("{name} has no {property}"));
        node.value.clone()
    };
    for (name, property, value) in [
        ("digikam/x-t2-colour-label.RAF.xmp", "ColorLabel", "1"),
        ("digikam/x-t2-tags.RAF.xmp", "ColorLabel", "3"),
        ("digikam/eos-7d-pick-label.JPG.xmp", "PickLabel", "3"),
        ("digikam/x-t2-pick-label.RAF.xmp", "PickLabel", "3"),
    ] {
        assert_eq!(label(name, property), XmpValue::Text(value.to_owned()));
    }
    let faces = parse("digikam/samsung-faces.jpg.xmp");
    assert!(
        faces
            .properties()
            .contains_key(&XmpName::new(MWG_REGIONS, "Regions"))
    );
}

/// Merges `Holiday/Lisbon` into `original`, writes and reads it, and checks that no property was lost.
fn assert_keyword_merge_keeps_every_property(name: &str, original: &XmpPacket) {
    let mut edited = original.clone();
    edited
        .add_keywords(&[&["Holiday".to_owned(), "Lisbon".to_owned()]])
        .unwrap();

    let read_back = write_and_read(&edited);

    let keyword_properties = [
        dc("subject"),
        XmpName::new(LIGHTROOM, "hierarchicalSubject"),
    ];
    let metadata_date = XmpName::new(XMP_BASIC, "MetadataDate");
    for (property, value) in original.properties() {
        let found = read_back.properties().get(property);
        if *property == metadata_date {
            continue; // every write stamps it
        }
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
