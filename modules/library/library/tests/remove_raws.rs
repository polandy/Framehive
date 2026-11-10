// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Remove RAWs: the plan, the toggle in the summary and the run with its failure
//! paths (LIBRARY-002).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod support;

use std::path::Path;

use library::{LibraryAsset, RemoveRawsPlan, RemoveRawsTotals, SkippedAssets};
use metadata::xmp::SidecarNaming;
use support::{Library, marked, path, paths, rated, rating_name};

const NAMING: SidecarNaming = SidecarNaming::OriginalWithExtension;

/// Adds an asset `base` with `extensions`, each of `size_bytes`.
fn add_asset(library: &Library, base: &str, extensions: &[&str], size_bytes: usize) {
    for extension in extensions {
        library.add_media(&format!("{base}.{extension}"), size_bytes);
    }
}

fn plan(library: &Library) -> RemoveRawsPlan {
    RemoveRawsPlan::new(&library.read().assets)
}

fn base_names<'a>(assets: impl Iterator<Item = &'a LibraryAsset>) -> Vec<&'a str> {
    assets.map(|asset| asset.asset.base_name()).collect()
}

/// spec: LIBRARY-002.9
#[test]
fn the_raw_and_its_own_sidecar_go_to_the_trash() {
    let library = Library::new();
    add_asset(&library, "DSCF1234", &["RAF", "JPG"], 30);
    library.add_sidecar("DSCF1234.RAF.xmp", &rated("4"), 0);
    library.add_sidecar("DSCF1234.JPG.xmp", &rated("4"), 0);
    let jpeg_sidecar = library.files.content(&path("DSCF1234.JPG.xmp"));

    let report = plan(&library).run(&library.trash);

    assert_eq!(
        library.trash.moved(),
        paths(&["DSCF1234.RAF", "DSCF1234.RAF.xmp"])
    );
    assert!(report.not_moved.is_empty());
    let asset = library.asset("DSCF1234");
    let files: Vec<_> = asset.asset.files().iter().map(|f| &f.path).collect();
    assert_eq!(
        files,
        paths(&["DSCF1234.JPG", "DSCF1234.JPG.xmp"])
            .iter()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        library.files.content(&path("DSCF1234.JPG.xmp")),
        jpeg_sidecar
    );
    let rating = library.packet("DSCF1234.JPG.xmp");
    assert_eq!(
        rating.property(&rating_name()),
        rated("4").property(&rating_name())
    );
}

/// spec: LIBRARY-002.10
#[test]
fn a_shared_sidecar_stays() {
    let library = Library::new();
    add_asset(&library, "DSCF1234", &["RAF", "JPG"], 30);
    library.add_sidecar("DSCF1234.xmp", &rated("4"), 0);
    let shared = library.files.content(&path("DSCF1234.xmp"));

    let _report = plan(&library).run(&library.trash);

    assert_eq!(library.trash.moved(), paths(&["DSCF1234.RAF"]));
    assert_eq!(library.files.content(&path("DSCF1234.xmp")), shared);
}

/// spec: LIBRARY-002.11
#[test]
fn marked_rejected_and_single_member_assets_are_skipped() {
    let library = Library::new();
    add_asset(&library, "A", &["RAF", "JPG"], 30);
    library.add_sidecar("A.xmp", &marked(), 0);
    add_asset(&library, "B", &["RAF", "JPG"], 30);
    library.add_sidecar("B.xmp", &rated("-1"), 0);
    add_asset(&library, "C", &["JPG"], 30);
    add_asset(&library, "D", &["RAF"], 30);
    add_asset(&library, "E", &["RAF", "JPG"], 30);

    let plan = plan(&library);
    let _report = plan.run(&library.trash);

    assert_eq!(library.trash.moved(), paths(&["E.RAF"]));
    assert_eq!(
        plan.skipped(),
        SkippedAssets {
            rejected: 1,
            rendered_only: 1,
            raw_only: 1,
        }
    );
    assert_eq!(base_names(plan.kept()), ["A"]);
}

/// spec: LIBRARY-002.12
#[test]
fn only_the_current_view() {
    let library = Library::new();
    add_asset(&library, "A", &["RAF", "JPG"], 30);
    add_asset(&library, "B", &["RAF", "JPG"], 30);
    let view: Vec<LibraryAsset> = library
        .read()
        .assets
        .into_iter()
        .filter(|asset| asset.asset.base_name() == "B")
        .collect();

    let _report = RemoveRawsPlan::new(&view).run(&library.trash);

    assert_eq!(library.trash.moved(), paths(&["B.RAF"]));
}

#[test]
fn the_plan_has_totals_and_two_groups() {
    let library = Library::new();
    add_asset(&library, "A", &["RAF"], 30_000);
    add_asset(&library, "A", &["JPG"], 9_000);
    library.add_sidecar("A.RAF.xmp", &rated("2"), 0);
    add_asset(&library, "B", &["RAF", "JPG"], 30_000);
    library.add_sidecar("B.xmp", &marked(), 0);
    add_asset(&library, "C", &["JPG"], 9_000);

    let plan = plan(&library);

    assert_eq!(
        plan.totals(),
        RemoveRawsTotals {
            assets: 1,
            files: 2,
            bytes: 30_000 + library_size(&library, "A.RAF.xmp"),
        }
    );
    assert_eq!(base_names(plan.removed()), ["A"]);
    assert_eq!(base_names(plan.kept()), ["B"]);
    assert_eq!(plan.skipped().rendered_only, 1);
    let files: Vec<_> = plan.files().map(|file| file.path.clone()).collect();
    assert_eq!(files, paths(&["A.RAF", "A.RAF.xmp"]));
}

#[test]
fn a_toggle_moves_the_asset_to_the_other_group_and_marks_it() {
    let library = Library::new();
    add_asset(&library, "A", &["RAF", "JPG"], 30);
    let mut plan = plan(&library);
    let id = path("A.RAF");

    assert_eq!(
        plan.toggle_keep_raw(library.sidecars(), NAMING, &id)
            .unwrap(),
        Some(true)
    );
    assert_eq!(base_names(plan.kept()), ["A"]);
    assert_eq!(plan.totals(), RemoveRawsTotals::default());
    assert!(library.asset("A").keep_raw);

    assert_eq!(
        plan.toggle_keep_raw(library.sidecars(), NAMING, &id)
            .unwrap(),
        Some(false)
    );
    assert_eq!(base_names(plan.removed()), ["A"]);
    assert!(!library.asset("A").keep_raw);
}

#[test]
fn a_toggle_of_an_asset_in_neither_group_does_nothing() {
    let library = Library::new();
    add_asset(&library, "C", &["JPG"], 30);
    let mut plan = plan(&library);

    let toggled = plan.toggle_keep_raw(library.sidecars(), NAMING, Path::new("/photos/C.JPG"));

    assert_eq!(toggled.unwrap(), None);
    assert!(!library.exists("C.JPG.xmp"));
}

/// spec: LIBRARY-002.15
#[test]
fn cancel_moves_no_file_and_keeps_the_toggled_markers() {
    let library = Library::new();
    add_asset(&library, "A", &["RAF", "JPG"], 30);
    add_asset(&library, "B", &["RAF", "JPG"], 30);
    library.add_sidecar("B.RAF.xmp", &marked(), 0);
    library.add_sidecar("B.JPG.xmp", &marked(), 0);
    let mut plan = plan(&library);

    plan.toggle_keep_raw(library.sidecars(), NAMING, &path("B.RAF"))
        .unwrap();
    drop(plan);

    assert!(library.trash.moved().is_empty());
    assert!(!library.asset("B").keep_raw);
}

/// spec: LIBRARY-002.16
#[test]
fn a_raw_that_cannot_be_moved_keeps_its_sidecar() {
    let library = Library::new();
    for base in ["A", "B"] {
        add_asset(&library, base, &["RAF", "JPG"], 30);
        library.add_sidecar(&format!("{base}.RAF.xmp"), &rated("3"), 0);
        library.add_sidecar(&format!("{base}.JPG.xmp"), &rated("3"), 0);
    }
    library.trash.refuse(&path("A.RAF"));
    let sidecar_bytes = library_size(&library, "B.RAF.xmp");

    let report = plan(&library).run(&library.trash);

    assert!(library.exists("A.RAF"));
    assert!(library.exists("A.RAF.xmp"));
    assert_eq!(library.trash.moved(), paths(&["B.RAF", "B.RAF.xmp"]));
    let not_moved: Vec<_> = report.not_moved.iter().map(|file| &file.path).collect();
    assert_eq!(not_moved, [&path("A.RAF")]);
    assert!(!report.not_moved[0].reason.to_string().is_empty());
    assert_eq!(
        report.moved,
        RemoveRawsTotals {
            assets: 1,
            files: 2,
            bytes: 30 + sidecar_bytes,
        }
    );
}

/// spec: LIBRARY-002.17
#[test]
fn a_sidecar_that_cannot_be_moved_is_reported() {
    let library = Library::new();
    add_asset(&library, "A", &["RAF", "JPG"], 30);
    library.add_sidecar("A.RAF.xmp", &rated("3"), 0);
    library.trash.refuse(&path("A.RAF.xmp"));

    let report = plan(&library).run(&library.trash);

    assert_eq!(library.trash.moved(), paths(&["A.RAF"]));
    assert!(library.exists("A.RAF.xmp"));
    let not_moved: Vec<_> = report.not_moved.iter().map(|file| &file.path).collect();
    assert_eq!(not_moved, [&path("A.RAF.xmp")]);
    assert_eq!(
        report.moved,
        RemoveRawsTotals {
            assets: 1,
            files: 1,
            bytes: 30,
        }
    );
}

fn library_size(library: &Library, name: &str) -> u64 {
    library.files.content(&path(name)).unwrap().len() as u64
}
