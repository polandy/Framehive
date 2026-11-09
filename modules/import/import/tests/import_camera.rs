// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `{camera}` token in an import (IMPORT-021).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod run_support;

use domain::{AssetFileRole, LocalDateTime};
use import::{AssetFile, ImportPlan};
use metadata::testing::{ByteOrder, DATE_TIME_ORIGINAL, MODEL, Tiff, jpeg};
use naming::{CompatibilityMode, NameTemplate};

use run_support::{MODIFIED, Setup, add, source_with, target};

const TEMPLATE: &str = "{camera}/{name}.{ext}";

fn file(file_name: &str, role: AssetFileRole, camera_model: Option<&str>) -> AssetFile {
    let time = LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap();
    AssetFile {
        file_name: file_name.to_owned(),
        role,
        capture_time: Some(time.into()),
        modification_time: time,
        camera_model: camera_model.map(str::to_owned),
    }
}

fn folders(files: Vec<AssetFile>) -> Vec<String> {
    let template = NameTemplate::parse(TEMPLATE).unwrap();
    let import_time = LocalDateTime::new(2026, 10, 31, 9, 0, 0).unwrap();
    let plan = ImportPlan::new(
        &[files],
        &template,
        None,
        CompatibilityMode::On,
        import_time,
        None,
        1,
    );
    let paths = plan.files().iter().map(|f| f.target.clone().unwrap());
    paths
        .map(|path| path.split('/').next().unwrap().to_owned())
        .collect()
}

/// spec: IMPORT-021.6
#[test]
fn the_raw_files_model_is_used_for_the_whole_asset() {
    let files = vec![
        file("DSC_0042.JPG", AssetFileRole::Rendered, Some("Other")),
        file("DSC_0042.RAF", AssetFileRole::Raw, Some("X-T5")),
    ];

    assert_eq!(folders(files), ["X-T5", "X-T5"]);
}

/// spec: IMPORT-021.6
#[test]
fn another_file_gives_the_model_when_the_raw_file_has_none() {
    let files = vec![
        file("DSC_0042.RAF", AssetFileRole::Raw, None),
        file("DSC_0042.JPG", AssetFileRole::Rendered, Some("X-T5")),
    ];

    assert_eq!(folders(files), ["X-T5", "X-T5"]);
}

/// spec: IMPORT-021.2
#[test]
fn an_asset_without_a_model_gets_the_fallback() {
    let files = vec![file("DSC_0042.JPG", AssetFileRole::Rendered, None)];

    assert_eq!(folders(files), ["Unknown camera"]);
}

/// spec: IMPORT-021.1
#[test]
fn the_import_reads_the_model_from_the_source_files() {
    let exif = Tiff::new(ByteOrder::LittleEndian)
        .with_ifd0_text(MODEL, "X-T5")
        .with_exif_text(DATE_TIME_ORIGINAL, "2026:10:17 14:05:09")
        .to_bytes();
    let file_system = source_with(&["plain.JPG"]);
    add(&file_system, "DSC_0042.JPG", &jpeg(&exif), MODIFIED);
    let setup = Setup {
        template: TEMPLATE,
        ..Setup::default()
    };

    let summary = setup.on(&file_system, &mut import::NoProgress);

    let targets: Vec<_> = summary.files.iter().map(target).collect();
    assert_eq!(
        targets,
        [Some("X-T5/DSC_0042.JPG"), Some("Unknown camera/plain.JPG")]
    );
}
