// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import plan made from a name template (IMPORT-001).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use domain::{AssetFileRole, LocalDateTime};
use import::{AssetFile, ImportPlan, JobCode, PlannedFile};
use naming::{CompatibilityMode, DateSource, NameTemplate, RenderError};

fn time(text: &str) -> LocalDateTime {
    // "YYYY-MM-DD HH:MM:SS", parsed here so the tests read like the spec.
    let number = |range: std::ops::Range<usize>| text[range].parse::<u8>().unwrap();
    let year = text[0..4].parse::<u16>().unwrap();
    LocalDateTime::new(
        year,
        number(5..7),
        number(8..10),
        number(11..13),
        number(14..16),
        number(17..19),
    )
    .unwrap()
}

fn import_time() -> LocalDateTime {
    time("2026-10-31 09:00:00")
}

/// A file with a capture time, modified a day later so a wrong choice shows.
fn captured(file_name: &str, role: AssetFileRole, capture_time: &str) -> AssetFile {
    AssetFile {
        file_name: file_name.to_owned(),
        role,
        capture_time: Some(time(capture_time).into()),
        modification_time: time("2026-12-24 12:00:00"),
    }
}

fn uncaptured(file_name: &str, role: AssetFileRole, modification_time: &str) -> AssetFile {
    AssetFile {
        file_name: file_name.to_owned(),
        role,
        capture_time: None,
        modification_time: time(modification_time),
    }
}

fn plan_with(
    template: &str,
    mode: CompatibilityMode,
    assets: &[Vec<AssetFile>],
) -> Vec<PlannedFile> {
    let template = NameTemplate::parse(template).unwrap();
    ImportPlan::new(assets, &template, mode, import_time(), None)
        .files()
        .to_vec()
}

fn plan(template: &str, assets: &[Vec<AssetFile>]) -> Vec<PlannedFile> {
    plan_with(template, CompatibilityMode::On, assets)
}

fn targets(files: &[PlannedFile]) -> Vec<Result<&str, &RenderError>> {
    files.iter().map(|file| file.target.as_deref()).collect()
}

/// spec: IMPORT-001.1
#[test]
fn folders_and_name_come_from_the_capture_time() {
    let raw = captured("DSC_0042.RAF", AssetFileRole::Raw, "2026-10-17 14:05:09");

    let files = plan(
        "{date:%Y}/{date:%Y-%m-%d}/{date:%Y%m%d}-{name}.{ext}",
        &[vec![raw]],
    );

    assert_eq!(
        targets(&files),
        [Ok("2026/2026-10-17/20261017-DSC_0042.RAF")]
    );
    assert_eq!(files[0].file_name, "DSC_0042.RAF");
    assert!(!files[0].is_dated_by_modification_time());
}

/// spec: IMPORT-001.5
#[test]
fn file_without_an_image_number_is_marked_and_the_others_are_planned() {
    let assets = [
        vec![captured(
            "IMG.JPG",
            AssetFileRole::Rendered,
            "2026-10-17 14:05:09",
        )],
        vec![captured(
            "DSC_0042.RAF",
            AssetFileRole::Raw,
            "2026-10-17 14:05:09",
        )],
    ];

    let files = plan("{number}.{ext}", &assets);

    let reason = files[0].target.as_ref().unwrap_err();
    assert!(
        matches!(reason, RenderError::NoImageNumber { token, .. } if token == "{number}"),
        "{reason:?}"
    );
    assert!(reason.to_string().contains("{number}"), "{reason}");
    assert_eq!(files[1].target.as_deref(), Ok("0042.RAF"));
}

/// spec: IMPORT-001.7
#[test]
fn file_without_capture_time_is_dated_by_its_modification_time() {
    let scan = uncaptured("scan.jpg", AssetFileRole::Rendered, "2026-03-01 08:00:00");

    let files = plan("{date:%Y-%m-%d}/{name}.{ext}", &[vec![scan]]);

    assert_eq!(targets(&files), [Ok("2026-03-01/scan.jpg")]);
    assert_eq!(files[0].date_source, DateSource::ModificationTime);
    assert!(files[0].is_dated_by_modification_time());
}

/// spec: IMPORT-001.8
#[test]
fn files_of_one_asset_share_the_capture_time() {
    let asset = vec![
        uncaptured(
            "DSC_0042.JPG",
            AssetFileRole::Rendered,
            "2026-10-18 00:00:01",
        ),
        captured("DSC_0042.RAF", AssetFileRole::Raw, "2026-10-17 23:59:59"),
    ];

    let files = plan("{date:%Y-%m-%d}/{name}.{ext}", &[asset]);

    assert_eq!(
        targets(&files),
        [Ok("2026-10-17/DSC_0042.JPG"), Ok("2026-10-17/DSC_0042.RAF")]
    );
    assert!(
        files
            .iter()
            .all(|file| file.date_source == DateSource::CaptureTime)
    );
}

/// spec: IMPORT-001.9
#[test]
fn sidecars_follow_their_file() {
    // The sidecar comes first, so the base name must come from the RAW file and not
    // from the first file of the asset.
    let asset = vec![
        uncaptured(
            "DSC_0042.RAF.xmp",
            AssetFileRole::Sidecar,
            "2026-10-20 10:00:00",
        ),
        captured("DSC_0042.RAF", AssetFileRole::Raw, "2026-10-17 14:05:09"),
        uncaptured(
            "DSC_0042.xmp",
            AssetFileRole::Sidecar,
            "2026-10-20 10:00:00",
        ),
    ];

    let files = plan("{date:%Y%m%d}-{number}.{ext}", &[asset]);

    assert_eq!(
        targets(&files),
        [
            Ok("20261017-0042.RAF.xmp"),
            Ok("20261017-0042.RAF"),
            Ok("20261017-0042.xmp"),
        ]
    );
}

/// spec: IMPORT-001.16
#[test]
fn compatibility_mode_replaces_invalid_characters() {
    let file = captured("what?.jpg", AssetFileRole::Rendered, "2026-10-17 14:05:00");

    let files = plan("{date:%H:%M} {name}. /{name}.{ext}", &[vec![file]]);

    assert_eq!(targets(&files), [Ok("14_05 what_/what_.jpg")]);
}

/// spec: IMPORT-001.19
#[test]
fn job_code_is_part_of_the_path() {
    let raw = captured("DSC_0042.RAF", AssetFileRole::Raw, "2026-10-17 14:05:09");
    let template = NameTemplate::parse("{date:%Y}/{job}/{name}.{ext}").unwrap();
    let job_code = JobCode::parse("Smith: wedding").unwrap();

    let plan = ImportPlan::new(
        &[vec![raw]],
        &template,
        CompatibilityMode::On,
        import_time(),
        Some(&job_code),
    );

    assert_eq!(
        targets(plan.files()),
        [Ok("2026/Smith_ wedding/DSC_0042.RAF")]
    );
}

/// spec: IMPORT-001.17
#[test]
fn compatibility_mode_off_keeps_the_values() {
    let file = captured("DSC_0042.RAF", AssetFileRole::Raw, "2026-10-17 14:05:00");

    let files = plan_with(
        "{date:%H:%M}/{name}.{ext}",
        CompatibilityMode::Off,
        &[vec![file]],
    );

    assert_eq!(targets(&files), [Ok("14:05/DSC_0042.RAF")]);
}

/// spec: IMPORT-001.18
#[test]
fn segment_that_would_leave_the_destination_is_marked_and_the_others_are_planned() {
    // `...` loses its trailing dots in compatibility mode and becomes empty.
    let assets = [
        vec![captured(
            "....jpg",
            AssetFileRole::Rendered,
            "2026-10-17 14:05:09",
        )],
        vec![captured(
            "DSC_0042.RAF",
            AssetFileRole::Raw,
            "2026-10-17 14:05:09",
        )],
    ];

    let files = plan("{name}/{date:%Y}.{ext}", &assets);

    assert_eq!(
        files[0].target,
        Err(RenderError::InvalidSegment {
            segment: String::new()
        })
    );
    assert_eq!(files[1].target.as_deref(), Ok("DSC_0042/2026.RAF"));
}

/// spec: IMPORT-001.18
#[test]
fn dot_segment_is_marked_without_compatibility_mode() {
    let file = captured("...jpg", AssetFileRole::Rendered, "2026-10-17 14:05:09");

    let files = plan_with(
        "{name}/{date:%Y}.{ext}",
        CompatibilityMode::Off,
        &[vec![file]],
    );

    assert_eq!(
        files[0].target,
        Err(RenderError::InvalidSegment {
            segment: "..".to_owned()
        })
    );
}

#[test]
fn every_input_file_gets_one_planned_file_in_order() {
    let assets = [
        vec![
            captured("DSC_0001.RAF", AssetFileRole::Raw, "2026-10-17 14:05:09"),
            captured(
                "DSC_0001.JPG",
                AssetFileRole::Rendered,
                "2026-10-17 14:05:09",
            ),
        ],
        Vec::new(),
        vec![captured(
            "DSC_0002.MOV",
            AssetFileRole::Video,
            "2026-10-17 14:06:00",
        )],
    ];

    let files = plan("{name}.{ext}", &assets);

    let names: Vec<&str> = files.iter().map(|file| file.file_name.as_str()).collect();
    assert_eq!(names, ["DSC_0001.RAF", "DSC_0001.JPG", "DSC_0002.MOV"]);
}

#[test]
fn asset_of_only_sidecars_takes_its_base_name_from_the_first_file() {
    let asset = vec![uncaptured(
        "notes.xmp",
        AssetFileRole::Sidecar,
        "2026-10-17 14:05:09",
    )];

    let files = plan("{date:%Y}/{name}.{ext}", &[asset]);

    assert_eq!(targets(&files), [Ok("2026/notes.xmp")]);
}

#[test]
fn import_time_is_the_one_the_plan_is_made_with() {
    let file = captured("DSC_0042.RAF", AssetFileRole::Raw, "2026-10-17 14:05:09");

    let files = plan("{imported:%Y-%m-%d}/{name}.{ext}", &[vec![file]]);

    assert_eq!(targets(&files), [Ok("2026-10-31/DSC_0042.RAF")]);
}
