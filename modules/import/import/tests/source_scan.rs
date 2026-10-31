// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The source scan (IMPORT-005), against `MemoryFileSystem`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::{Path, PathBuf};

use domain::{AssetFileRole, Timestamp};
use files::testing::{Fault, MemoryFileSystem};
use import::{LeftOut, LeftOutReason, ScanError, SourceScan, scan_source};

const SOURCE: &str = "/media/card";
const DESTINATION: &str = "/library";

/// 2026-10-17 14:05:09 UTC.
const MODIFIED: i64 = 1_792_245_909;

fn source_with(files: &[&str]) -> MemoryFileSystem {
    let file_system = MemoryFileSystem::new();
    file_system.add_folder(Path::new(SOURCE));
    for file in files {
        add(&file_system, file, b"content");
    }
    file_system
}

fn add(file_system: &MemoryFileSystem, relative: &str, content: &[u8]) {
    let modified = Timestamp::from_unix_seconds(MODIFIED);
    file_system.add_file(&Path::new(SOURCE).join(relative), content, modified);
}

fn scan(file_system: &MemoryFileSystem) -> SourceScan {
    scan_source(file_system, Path::new(SOURCE), Path::new(DESTINATION)).unwrap()
}

fn found(scan: &SourceScan) -> Vec<&str> {
    scan.files
        .iter()
        .map(|file| file.path.to_str().unwrap())
        .collect()
}

fn left_out(path: &str, reason: LeftOutReason) -> LeftOut {
    LeftOut {
        path: PathBuf::from(path),
        reason,
    }
}

/// spec: IMPORT-005.1
#[test]
fn whole_source_is_scanned() {
    let file_system = source_with(&["DCIM/100_FUJI/DSCF0001.RAF", "copied/holiday.jpg"]);

    let scan = scan(&file_system);

    assert_eq!(
        found(&scan),
        ["DCIM/100_FUJI/DSCF0001.RAF", "copied/holiday.jpg"]
    );
    assert_eq!(scan.left_out, []);
}

/// spec: IMPORT-005.2
#[test]
fn roles_come_from_the_extension_ignoring_case() {
    let file_system = source_with(&["DSCF0001.RAF", "photo.jpeg", "clip.MOV"]);

    let scan = scan(&file_system);

    let roles: Vec<(&str, AssetFileRole)> = scan
        .files
        .iter()
        .map(|file| (file.path.to_str().unwrap(), file.role))
        .collect();
    assert_eq!(
        roles,
        [
            ("DSCF0001.RAF", AssetFileRole::Raw),
            ("clip.MOV", AssetFileRole::Video),
            ("photo.jpeg", AssetFileRole::Rendered),
        ]
    );
}

/// spec: IMPORT-005.3
#[test]
fn unsupported_files_are_listed_not_imported() {
    let file_system = source_with(&["DSCF0001.RAF", "notes.txt", "README"]);

    let scan = scan(&file_system);

    assert_eq!(found(&scan), ["DSCF0001.RAF"]);
    assert_eq!(
        scan.left_out,
        [
            left_out("README", LeftOutReason::UnsupportedType),
            left_out("notes.txt", LeftOutReason::UnsupportedType),
        ]
    );
    assert_eq!(scan.left_out[0].reason.to_string(), "unsupported type");
}

/// spec: IMPORT-005.4
#[test]
fn hidden_files_and_folders_are_left_out() {
    let file_system = source_with(&["DSCF0001.RAF", "._DSCF0001.RAF", ".Trashes/DSCF0002.RAF"]);

    let scan = scan(&file_system);

    assert_eq!(found(&scan), ["DSCF0001.RAF"]);
    // The hidden folder is listed once and not entered.
    assert_eq!(
        scan.left_out,
        [
            left_out(".Trashes", LeftOutReason::Hidden),
            left_out("._DSCF0001.RAF", LeftOutReason::Hidden),
        ]
    );
    assert_eq!(scan.left_out[0].reason.to_string(), "hidden");
}

/// spec: IMPORT-005.5
#[test]
fn symbolic_links_are_not_followed() {
    let file_system = source_with(&["DSCF0001.RAF"]);
    file_system.add_symbolic_link(&Path::new(SOURCE).join("all"));
    file_system.add_symbolic_link(&Path::new(SOURCE).join("photo.jpg"));

    let scan = scan(&file_system);

    assert_eq!(found(&scan), ["DSCF0001.RAF"]);
    assert_eq!(
        scan.left_out,
        [
            left_out("all", LeftOutReason::SymbolicLink),
            left_out("photo.jpg", LeftOutReason::SymbolicLink),
        ]
    );
    assert_eq!(scan.left_out[0].reason.to_string(), "symbolic link");
}

/// spec: IMPORT-005.6
#[test]
fn unreadable_folder_does_not_stop_the_scan() {
    let file_system = source_with(&["DSCF0001.RAF", "locked/DSCF0002.RAF"]);
    file_system.inject_fault(&Path::new(SOURCE).join("locked"), Fault::ListFails);

    let scan = scan(&file_system);

    assert_eq!(found(&scan), ["DSCF0001.RAF"]);
    assert_eq!(scan.left_out.len(), 1);
    assert_eq!(scan.left_out[0].path, Path::new("locked"));
    assert!(matches!(
        &scan.left_out[0].reason,
        LeftOutReason::CannotBeRead(error) if error.contains("cannot be read")
    ));
}

/// spec: IMPORT-005.7
#[test]
fn order_is_stable() {
    let file_system = source_with(&["b/2.jpg", "a/1.jpg", "B.jpg"]);

    for _ in 0..3 {
        assert_eq!(found(&scan(&file_system)), ["B.jpg", "a/1.jpg", "b/2.jpg"]);
    }
}

#[test]
fn order_is_the_byte_order_of_the_relative_paths() {
    // `.` (0x2E) sorts before `/` (0x2F), unlike a comparison by path components.
    let file_system = source_with(&["a/1.jpg", "a.jpg"]);

    assert_eq!(found(&scan(&file_system)), ["a.jpg", "a/1.jpg"]);
}

/// spec: IMPORT-005.8
#[test]
fn size_and_modification_time_are_given() {
    let file_system = source_with(&[]);
    add(&file_system, "DSCF0001.RAF", &[0; 1_024]);

    let scan = scan(&file_system);

    assert_eq!(scan.files[0].size, 1_024);
    assert_eq!(
        scan.files[0].modification_time,
        Timestamp::from_unix_seconds(MODIFIED)
    );
}

/// spec: IMPORT-005.9
#[test]
fn empty_source_is_not_an_error() {
    let file_system = source_with(&[]);

    assert_eq!(scan(&file_system), SourceScan::default());
}

/// spec: IMPORT-005.10
#[test]
fn missing_source_fails_the_scan() {
    let file_system = source_with(&["DSCF0001.RAF"]);
    let missing = Path::new("/media/other");
    let file = Path::new(SOURCE).join("DSCF0001.RAF");

    let missing_error = scan_source(&file_system, missing, Path::new(DESTINATION)).unwrap_err();
    let file_error = scan_source(&file_system, &file, Path::new(DESTINATION)).unwrap_err();

    assert!(matches!(missing_error, ScanError::SourceMissing { .. }));
    assert!(missing_error.to_string().contains("/media/other"));
    assert!(matches!(file_error, ScanError::SourceNotAFolder { .. }));
    assert!(file_error.to_string().contains("/media/card/DSCF0001.RAF"));
}

/// spec: IMPORT-005.11
#[test]
fn destination_inside_the_source_is_refused() {
    let file_system = source_with(&["DSCF0001.RAF"]);
    let paths_before = file_system.paths();

    let error = scan_source(
        &file_system,
        Path::new("/media/card"),
        Path::new("/media/card/library"),
    )
    .unwrap_err();

    assert!(matches!(error, ScanError::DestinationInsideSource { .. }));
    let message = error.to_string();
    assert!(message.contains("/media/card/library"), "{message}");
    assert!(message.contains("the source /media/card"), "{message}");
    // Refused before the scan: the file system was not touched.
    assert_eq!(file_system.operations(), 0);
    assert_eq!(file_system.paths(), paths_before);
}

#[test]
fn destination_equal_to_the_source_is_refused() {
    let file_system = source_with(&[]);

    let error = scan_source(&file_system, Path::new(SOURCE), Path::new("/media/card/."));

    assert!(matches!(
        error,
        Err(ScanError::DestinationInsideSource { .. })
    ));
}

#[test]
fn destination_next_to_the_source_is_allowed() {
    let file_system = source_with(&["DSCF0001.RAF"]);

    let scan = scan_source(&file_system, Path::new(SOURCE), Path::new("/media/cards"));

    assert_eq!(found(&scan.unwrap()), ["DSCF0001.RAF"]);
}

/// spec: IMPORT-005.12
#[test]
fn sidecars_are_left_out_until_grouping() {
    let file_system = source_with(&["DSCF0001.RAF", "DSCF0001.xmp", "clip.Thm"]);

    let scan = scan(&file_system);

    assert_eq!(found(&scan), ["DSCF0001.RAF"]);
    assert_eq!(
        scan.left_out,
        [
            left_out("DSCF0001.xmp", LeftOutReason::SidecarNeedsGrouping),
            left_out("clip.Thm", LeftOutReason::SidecarNeedsGrouping),
        ]
    );
    assert_eq!(
        scan.left_out[0].reason.to_string(),
        "sidecar, needs grouping"
    );
}

#[test]
fn every_listed_extension_has_its_role() {
    let cases = [
        (
            "3FR ARW CR2 CR3 CRW DCR DNG FFF IIQ MEF MOS MRW NEF NRW ORF PEF RAF RAW RW2 SR2 SRW X3F",
            AssetFileRole::Raw,
        ),
        (
            "JPG JPEG HEIF HEIC HIF TIF TIFF MPO AVIF JXL PNG",
            AssetFileRole::Rendered,
        ),
        (
            "3GP AVI M2T M2TS MOV MP4 MPEG MPG MOD TOD MTS MKV",
            AssetFileRole::Video,
        ),
    ];
    for (extensions, role) in cases {
        let names: Vec<String> = extensions
            .split(' ')
            .map(|extension| format!("file.{}", extension.to_lowercase()))
            .collect();
        let file_system = source_with(&names.iter().map(String::as_str).collect::<Vec<_>>());

        let scan = scan(&file_system);

        assert_eq!(scan.files.len(), names.len(), "{extensions}");
        assert!(
            scan.files.iter().all(|file| file.role == role),
            "{extensions}"
        );
    }
}
