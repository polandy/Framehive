// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The grouping rule: which files of a folder form one asset (LIBRARY-001).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use std::path::{Path, PathBuf};

use domain::AssetFileRole;
use library::{AmbiguousBaseName, Asset, FolderFile, Grouping, group_files};

fn group(folder: &str, names: &[&str]) -> Grouping {
    let files: Vec<FolderFile> = names
        .iter()
        .map(|name| FolderFile {
            name: (*name).to_owned(),
            size_bytes: 1,
        })
        .collect();
    group_files(Path::new(folder), &files)
}

/// The names and roles of the files of each asset.
fn members(grouping: &Grouping) -> Vec<Vec<(String, AssetFileRole)>> {
    grouping.assets.iter().map(names_and_roles).collect()
}

fn names_and_roles(asset: &Asset) -> Vec<(String, AssetFileRole)> {
    asset
        .files()
        .iter()
        .map(|file| {
            let name = file.path.file_name().unwrap_or_default();
            (name.to_string_lossy().into_owned(), file.role)
        })
        .collect()
}

fn member(name: &str, role: AssetFileRole) -> (String, AssetFileRole) {
    (name.to_owned(), role)
}

/// spec: LIBRARY-001.1
#[test]
fn raw_and_jpeg_form_one_asset() {
    let grouping = group("/photos", &["DSCF1234.JPG", "DSCF1234.RAF"]);

    assert_eq!(
        members(&grouping),
        [[
            member("DSCF1234.RAF", AssetFileRole::Raw),
            member("DSCF1234.JPG", AssetFileRole::Rendered),
        ]]
    );
    assert_eq!(grouping.assets[0].base_name(), "DSCF1234");
    assert!(grouping.ambiguous.is_empty());
}

/// spec: LIBRARY-001.2
#[test]
fn heif_pairs_like_jpeg() {
    let grouping = group("/photos", &["DSCF1234.RAF", "DSCF1234.HIF"]);

    assert_eq!(
        members(&grouping),
        [[
            member("DSCF1234.RAF", AssetFileRole::Raw),
            member("DSCF1234.HIF", AssetFileRole::Rendered),
        ]]
    );
}

/// spec: LIBRARY-001.3
#[test]
fn different_folders_do_not_group() {
    let a = group("/photos/a", &["DSCF1234.RAF"]);
    let b = group("/photos/b", &["DSCF1234.JPG"]);

    assert_eq!(a.assets.len() + b.assets.len(), 2);
    assert_eq!(a.assets[0].id(), Path::new("/photos/a/DSCF1234.RAF"));
    assert_eq!(b.assets[0].id(), Path::new("/photos/b/DSCF1234.JPG"));
}

/// spec: LIBRARY-001.4
#[test]
fn ambiguous_members_do_not_group() {
    let grouping = group("/photos", &["A.RAF", "A.JPG", "A.jpg"]);

    assert_eq!(
        members(&grouping),
        [
            vec![member("A.JPG", AssetFileRole::Rendered)],
            vec![member("A.RAF", AssetFileRole::Raw)],
            vec![member("A.jpg", AssetFileRole::Rendered)],
        ]
    );
    assert_eq!(
        grouping.ambiguous,
        [AmbiguousBaseName {
            folder: PathBuf::from("/photos"),
            base_name: "A".to_owned(),
        }]
    );
}

#[test]
fn base_names_are_compared_case_sensitively() {
    let grouping = group("/photos", &["dscf1234.RAF", "DSCF1234.JPG"]);

    assert_eq!(grouping.assets.len(), 2);
    assert!(grouping.ambiguous.is_empty());
}

#[test]
fn extensions_are_compared_case_insensitively() {
    let grouping = group("/photos", &["DSCF1234.raf", "DSCF1234.jpeg"]);

    assert_eq!(grouping.assets.len(), 1);
}

#[test]
fn own_and_shared_sidecars_join_their_asset() {
    let grouping = group(
        "/photos",
        &["A.RAF", "A.JPG", "A.RAF.xmp", "A.JPG.XMP", "A.xmp", "B.JPG"],
    );

    assert_eq!(
        members(&grouping),
        [
            vec![
                member("A.RAF", AssetFileRole::Raw),
                member("A.JPG", AssetFileRole::Rendered),
                member("A.JPG.XMP", AssetFileRole::Sidecar),
                member("A.RAF.xmp", AssetFileRole::Sidecar),
                member("A.xmp", AssetFileRole::Sidecar),
            ],
            vec![member("B.JPG", AssetFileRole::Rendered)],
        ]
    );
    let asset = &grouping.assets[0];
    let raw = asset.file(AssetFileRole::Raw).unwrap();
    let own: Vec<_> = asset.own_sidecars(raw).map(|file| &file.path).collect();
    assert_eq!(own, [Path::new("/photos/A.RAF.xmp")]);
    assert_eq!(
        asset.shared_sidecar().map(|file| file.path.as_path()),
        Some(Path::new("/photos/A.xmp"))
    );
}

#[test]
fn a_shared_sidecar_of_an_ambiguous_base_name_joins_no_asset() {
    let grouping = group("/photos", &["A.JPG", "A.jpg", "A.xmp", "A.JPG.xmp"]);

    assert_eq!(
        members(&grouping),
        [
            vec![
                member("A.JPG", AssetFileRole::Rendered),
                member("A.JPG.xmp", AssetFileRole::Sidecar),
            ],
            vec![member("A.jpg", AssetFileRole::Rendered)],
        ]
    );
}

#[test]
fn a_video_is_an_asset_with_its_thm_and_does_not_pair() {
    let grouping = group("/photos", &["MVI_0001.MOV", "MVI_0001.THM", "MVI_0001.JPG"]);

    assert_eq!(
        members(&grouping),
        [
            vec![
                member("MVI_0001.MOV", AssetFileRole::Video),
                member("MVI_0001.THM", AssetFileRole::Sidecar),
            ],
            vec![member("MVI_0001.JPG", AssetFileRole::Rendered)],
        ]
    );
    assert!(grouping.ambiguous.is_empty());
}

#[test]
fn files_of_other_types_and_lone_sidecars_are_not_part_of_the_library() {
    let grouping = group("/photos", &["notes.txt", "README", "Z.RAF.xmp", "Z.THM"]);

    assert!(grouping.assets.is_empty());
}
