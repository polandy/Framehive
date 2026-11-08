// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The scan attaches each source `.xmp` sidecar to the file it names (IMPORT-020),
//! against `MemoryFileSystem`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::{Path, PathBuf};

use domain::Timestamp;
use files::testing::MemoryFileSystem;
use import::{LeftOut, LeftOutReason, SourceScan, SourceSidecar, scan_folder};
use metadata::xmp::SidecarNaming;

const SOURCE: &str = "/media/card";

fn scan_of(files: &[&str]) -> SourceScan {
    let file_system = MemoryFileSystem::new();
    file_system.add_folder(Path::new(SOURCE));
    for file in files {
        let modified = Timestamp::from_unix_seconds(1_792_245_909);
        file_system.add_file(&Path::new(SOURCE).join(file), b"content", modified);
    }
    scan_folder(&file_system, Path::new(SOURCE)).unwrap()
}

/// Each file found with the paths of its sidecars.
fn attached(scan: &SourceScan) -> Vec<(&str, Vec<&str>)> {
    scan.files
        .iter()
        .map(|file| {
            let sidecars = file
                .sidecars
                .iter()
                .map(|sidecar| sidecar.path.to_str().unwrap())
                .collect();
            (file.path.to_str().unwrap(), sidecars)
        })
        .collect()
}

fn left_out(path: &str, reason: LeftOutReason) -> LeftOut {
    LeftOut {
        path: PathBuf::from(path),
        reason,
    }
}

/// spec: IMPORT-020.1
#[test]
fn a_sidecar_named_after_the_full_name_comes_with_its_file() {
    let scan = scan_of(&["DCIM/DSCF0042.RAF", "DCIM/DSCF0042.RAF.xmp"]);

    assert_eq!(
        scan.files[0].sidecars,
        [SourceSidecar {
            path: PathBuf::from("DCIM/DSCF0042.RAF.xmp"),
            naming: SidecarNaming::OriginalWithExtension,
            size: 7,
        }]
    );
    assert!(scan.left_out.is_empty());
}

/// spec: IMPORT-020.2
#[test]
fn a_sidecar_without_extension_goes_to_the_raw() {
    let scan = scan_of(&["DSCF0042.JPG", "DSCF0042.RAF", "DSCF0042.xmp"]);

    assert_eq!(
        attached(&scan),
        [
            ("DSCF0042.JPG", vec![]),
            ("DSCF0042.RAF", vec!["DSCF0042.xmp"])
        ]
    );
    let naming = scan.files[1].sidecars[0].naming;
    assert_eq!(naming, SidecarNaming::OriginalWithoutExtension);
}

#[test]
fn the_extensions_are_matched_in_any_case() {
    let scan = scan_of(&["DSCF0042.raf", "DSCF0042.RAF.XMP", "clip.MOV", "clip.Xmp"]);

    assert_eq!(
        attached(&scan),
        [
            ("DSCF0042.raf", vec!["DSCF0042.RAF.XMP"]),
            ("clip.MOV", vec!["clip.Xmp"])
        ]
    );
}

#[test]
fn a_file_carries_one_sidecar_of_each_naming_form() {
    let scan = scan_of(&["DSCF0042.RAF", "DSCF0042.RAF.xmp", "DSCF0042.xmp"]);

    assert_eq!(
        attached(&scan),
        [("DSCF0042.RAF", vec!["DSCF0042.RAF.xmp", "DSCF0042.xmp"])]
    );
}

#[test]
fn a_sidecar_attaches_only_in_its_own_folder() {
    let scan = scan_of(&["100_FUJI/DSCF0042.RAF", "101_FUJI/DSCF0042.RAF.xmp"]);

    assert_eq!(attached(&scan), [("100_FUJI/DSCF0042.RAF", vec![])]);
    assert_eq!(
        scan.left_out,
        [left_out(
            "101_FUJI/DSCF0042.RAF.xmp",
            LeftOutReason::SidecarWithoutItsFile
        )]
    );
}

/// spec: IMPORT-020.3
#[test]
fn a_sidecar_without_its_file_is_left_out() {
    let scan = scan_of(&["DSCF0041.RAF", "DSCF0042.xmp"]);

    assert_eq!(attached(&scan), [("DSCF0041.RAF", vec![])]);
    assert_eq!(
        scan.left_out,
        [left_out(
            "DSCF0042.xmp",
            LeftOutReason::SidecarWithoutItsFile
        )]
    );
    assert_eq!(
        scan.left_out[0].reason.to_string(),
        "sidecar without its file"
    );
}

/// spec: IMPORT-020.4
#[test]
fn thm_files_stay_left_out() {
    let scan = scan_of(&["clip.MOV", "clip.THM"]);

    assert_eq!(attached(&scan), [("clip.MOV", vec![])]);
    assert_eq!(
        scan.left_out,
        [left_out("clip.THM", LeftOutReason::SidecarNeedsGrouping)]
    );
    assert_eq!(
        scan.left_out[0].reason.to_string(),
        "sidecar, needs grouping"
    );
}
