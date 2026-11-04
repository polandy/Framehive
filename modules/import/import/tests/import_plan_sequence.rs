// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sequence numbers in the import plan: per asset, in shooting order (IMPORT-002).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use domain::{AssetFileRole, LocalDateTime};
use import::{AssetFile, ImportPlan, PlannedFile};
use naming::{CompatibilityMode, NameTemplate, RenderError};

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

fn captured(file_name: &str, role: AssetFileRole, capture_time: &str) -> AssetFile {
    AssetFile {
        file_name: file_name.to_owned(),
        role,
        capture_time: Some(time(capture_time).into()),
        modification_time: time("2026-12-24 12:00:00"),
    }
}

fn targets(files: &[PlannedFile]) -> Vec<Result<&str, &RenderError>> {
    files.iter().map(|file| file.target.as_deref()).collect()
}

fn numbered(template: &str, assets: &[Vec<AssetFile>], first: u32) -> Vec<PlannedFile> {
    let template = NameTemplate::parse(template).unwrap();
    ImportPlan::new(
        assets,
        &template,
        CompatibilityMode::On,
        import_time(),
        None,
        first,
    )
    .files()
    .to_vec()
}

/// spec: IMPORT-002.3
#[test]
fn files_of_one_asset_share_a_sequence_number() {
    let assets = [
        vec![
            captured("DSC_0001.ARW", AssetFileRole::Raw, "2026-10-17 14:05:09"),
            captured(
                "DSC_0001.JPG",
                AssetFileRole::Rendered,
                "2026-10-17 14:05:09",
            ),
        ],
        vec![captured(
            "DSC_0002.ARW",
            AssetFileRole::Raw,
            "2026-10-17 14:05:10",
        )],
    ];

    let files = numbered("{seq_session:4}.{ext}", &assets, 1);

    assert_eq!(
        targets(&files),
        [Ok("0001.ARW"), Ok("0001.JPG"), Ok("0002.ARW")]
    );
}

/// spec: IMPORT-002.12
#[test]
fn assets_are_numbered_in_shooting_order() {
    let assets = [
        vec![captured("C.RAF", AssetFileRole::Raw, "2026-10-17 10:00:05")],
        vec![captured("A.RAF", AssetFileRole::Raw, "2026-10-17 10:00:05")],
        vec![captured("B.RAF", AssetFileRole::Raw, "2026-10-17 10:00:00")],
    ];

    let files = numbered("{seq_session:1}.{ext}", &assets, 1);

    // The plan keeps the order of the assets; only the numbers follow the shooting.
    assert_eq!(targets(&files), [Ok("3.RAF"), Ok("2.RAF"), Ok("1.RAF")]);
}

#[test]
fn the_range_starts_at_the_first_sequence_number() {
    let assets = [
        vec![captured("A.RAF", AssetFileRole::Raw, "2026-10-17 10:00:00")],
        vec![captured("B.RAF", AssetFileRole::Raw, "2026-10-17 10:00:01")],
    ];

    let files = numbered("{seq_stored:4}.{ext}", &assets, 251);

    assert_eq!(targets(&files), [Ok("0251.RAF"), Ok("0252.RAF")]);
}
