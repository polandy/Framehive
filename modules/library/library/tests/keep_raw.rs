// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Keep RAW marker and the rejected state in the sidecars of an asset's members,
//! and reading them back (LIBRARY-001, LIBRARY-002).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod support;

use library::set_keep_raw;
use metadata::xmp::namespaces::FRAMEHIVE;
use metadata::xmp::{SidecarErrorKind, SidecarNaming, XmpName};
use support::{Library, marked, path, rated, rating_name};

/// spec: LIBRARY-002.7
#[test]
fn marking_writes_every_members_sidecar_and_survives_a_rebuild() {
    let library = Library::new();
    library.add_media("DSCF1234.RAF", 30);
    library.add_media("DSCF1234.JPG", 10);

    let asset = library.asset("DSCF1234");
    set_keep_raw(
        library.sidecars(),
        SidecarNaming::OriginalWithExtension,
        &asset.asset,
        true,
    )
    .unwrap();

    assert!(library.packet("DSCF1234.RAF.xmp").keep_raw());
    assert!(library.packet("DSCF1234.JPG.xmp").keep_raw());
    assert!(library.asset("DSCF1234").keep_raw);
}

#[test]
fn naming_without_extension_writes_one_shared_sidecar() {
    let library = Library::new();
    library.add_media("DSCF1234.RAF", 30);
    library.add_media("DSCF1234.JPG", 10);

    let asset = library.asset("DSCF1234");
    set_keep_raw(
        library.sidecars(),
        SidecarNaming::OriginalWithoutExtension,
        &asset.asset,
        true,
    )
    .unwrap();

    assert!(library.packet("DSCF1234.xmp").keep_raw());
    assert!(!library.exists("DSCF1234.RAF.xmp"));
    assert!(!library.exists("DSCF1234.JPG.xmp"));
}

#[test]
fn unmarking_keeps_the_other_properties() {
    let library = Library::new();
    library.add_media("DSCF1234.RAF", 30);
    library.add_media("DSCF1234.JPG", 10);
    let mut packet = rated("4");
    packet.set_keep_raw(true).unwrap();
    library.add_sidecar("DSCF1234.RAF.xmp", &packet, 0);
    library.add_sidecar("DSCF1234.JPG.xmp", &packet, 0);

    let asset = library.asset("DSCF1234");
    assert!(asset.keep_raw);
    set_keep_raw(
        library.sidecars(),
        SidecarNaming::OriginalWithExtension,
        &asset.asset,
        false,
    )
    .unwrap();

    for name in ["DSCF1234.RAF.xmp", "DSCF1234.JPG.xmp"] {
        let packet = library.packet(name);
        assert!(!packet.keep_raw());
        assert_eq!(
            packet.property(&rating_name()),
            rated("4").property(&rating_name())
        );
    }
}

#[test]
fn a_marker_of_another_form_refuses_the_change_for_every_member() {
    let library = Library::new();
    library.add_media("DSCF1234.RAF", 30);
    library.add_media("DSCF1234.JPG", 10);
    let mut array = rated("4");
    array
        .replace_seq(&XmpName::new(FRAMEHIVE, "KeepRaw"), &["True"])
        .unwrap();
    library.add_sidecar("DSCF1234.JPG.xmp", &array, 0);
    let before = library.files.content(&path("DSCF1234.JPG.xmp"));

    let asset = library.asset("DSCF1234");
    let error = set_keep_raw(
        library.sidecars(),
        SidecarNaming::OriginalWithExtension,
        &asset.asset,
        true,
    )
    .unwrap_err();

    assert!(matches!(error.kind, SidecarErrorKind::Edit(_)));
    assert_eq!(error.path, path("DSCF1234.JPG.xmp"));
    assert!(!library.exists("DSCF1234.RAF.xmp"));
    assert_eq!(library.files.content(&path("DSCF1234.JPG.xmp")), before);
}

#[test]
fn the_newest_sidecar_holds_the_state() {
    let library = Library::new();
    library.add_media("A.RAF", 30);
    library.add_media("A.JPG", 10);
    library.add_sidecar("A.RAF.xmp", &rated("2"), 10);
    library.add_sidecar("A.JPG.xmp", &marked(), 11);

    assert!(library.asset("A").keep_raw);
}

#[test]
fn on_a_tie_the_raws_own_sidecar_holds_the_state() {
    let library = Library::new();
    library.add_media("A.RAF", 30);
    library.add_media("A.JPG", 10);
    library.add_sidecar("A.RAF.xmp", &marked(), 10);
    library.add_sidecar("A.JPG.xmp", &rated("2"), 10);
    library.add_sidecar("A.xmp", &rated("2"), 10);

    assert!(library.asset("A").keep_raw);
}

#[test]
fn a_rating_of_minus_one_is_rejected() {
    let library = Library::new();
    library.add_media("A.RAF", 30);
    library.add_media("A.JPG", 10);
    library.add_sidecar("A.xmp", &rated("-1"), 0);

    let asset = library.asset("A");
    assert!(asset.rejected);
    assert!(!asset.keep_raw);
}

#[test]
fn an_asset_with_an_unreadable_sidecar_is_left_out() {
    let library = Library::new();
    library.add_media("A.RAF", 30);
    library.add_media("A.JPG", 10);
    library.files.add_file(
        &path("A.JPG.xmp"),
        b"not XMP",
        domain::Timestamp::UNIX_EPOCH,
    );
    library.add_media("B.JPG", 10);

    let folder = library.read();

    let names: Vec<_> = folder.assets.iter().map(|a| a.asset.base_name()).collect();
    assert_eq!(names, ["B"]);
    assert_eq!(folder.unreadable.len(), 1);
    assert_eq!(folder.unreadable[0].path, path("A.JPG.xmp"));
}
