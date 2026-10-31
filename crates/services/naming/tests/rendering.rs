// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rendering of name templates for the files of an asset (IMPORT-001).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use domain::{AssetFileRole, LocalDateTime};
use naming::{
    AssetValues, CompatibilityMode, DateSource, FileTimes, NameTemplate, RenderError, asset_date,
};

fn time(text: &str) -> LocalDateTime {
    // "YYYY-MM-DD HH:MM:SS", parsed here so the tests read like the spec.
    let number = |range: std::ops::Range<usize>| text[range].parse::<u8>().unwrap();
    LocalDateTime::new(
        text[0..4].parse().unwrap(),
        number(5..7),
        number(8..10),
        number(11..13),
        number(14..16),
        number(17..19),
    )
    .unwrap()
}

fn values(base_name: &str, capture_time: &str) -> AssetValues {
    AssetValues {
        base_name: base_name.to_owned(),
        capture_time: time(capture_time),
        import_time: time("2026-10-19 09:00:00"),
    }
}

fn render(template: &str, values: &AssetValues, file_name: &str) -> Result<String, RenderError> {
    NameTemplate::parse(template)
        .unwrap()
        .render(values, file_name, CompatibilityMode::On)
}

/// spec: IMPORT-001.1
#[test]
fn folders_and_name_come_from_the_capture_time() {
    let values = values("DSC_0042", "2026-10-17 14:05:09");

    let path = render(
        "{date:%Y}/{date:%Y-%m-%d}/{date:%Y%m%d}-{name}.{ext}",
        &values,
        "DSC_0042.RAF",
    );

    assert_eq!(path.unwrap(), "2026/2026-10-17/20261017-DSC_0042.RAF");
}

/// spec: IMPORT-001.1
#[test]
fn every_date_code_is_rendered() {
    let values = values("DSC_0042", "2026-10-07 04:05:09");

    let path = render("{date:%y|%j|%H|%M|%S|%%}.{ext}", &values, "DSC_0042.RAF");

    // Compatibility mode replaces the `|`; the codes themselves are what is checked.
    assert_eq!(path.unwrap(), "26_280_04_05_09_%.RAF");
}

/// spec: IMPORT-001.1
#[test]
fn day_of_year_counts_the_leap_day() {
    let values = values("A", "2024-12-31 00:00:00");

    assert_eq!(
        render("{date:%j}.{ext}", &values, "A.JPG").unwrap(),
        "366.JPG"
    );
}

/// spec: IMPORT-001.2
#[test]
fn literal_text_and_escaped_braces_are_kept() {
    let values = values("DSC_0042", "2026-10-17 14:05:09");

    let name = render("{{draft}}_{name}.{ext}", &values, "DSC_0042.RAF");

    assert_eq!(name.unwrap(), "{draft}_DSC_0042.RAF");
}

/// spec: IMPORT-001.3
#[test]
fn case_options_change_name_and_extension() {
    let values = values("DSC_0042", "2026-10-17 14:05:09");

    let name = render("{name:lower}.{ext:upper}", &values, "DSC_0042.RAF");
    assert_eq!(name.unwrap(), "dsc_0042.RAF");

    let lower = values_with_base("dsc_0042");
    assert_eq!(
        render("{name:upper}.{ext:lower}", &lower, "dsc_0042.RAF").unwrap(),
        "DSC_0042.raf"
    );
}

#[test]
fn file_of_another_asset_is_refused() {
    let values = values_with_base("DSC_0042");

    for file_name in ["dsc_0042.raf", "DSC_0043.RAF", "DSC_0042"] {
        assert_eq!(
            render("{name}.{ext}", &values, file_name),
            Err(RenderError::FileNotInAsset {
                file_name: file_name.to_owned(),
                base_name: "DSC_0042".to_owned(),
            })
        );
    }
}

fn values_with_base(base_name: &str) -> AssetValues {
    values(base_name, "2026-10-17 14:05:09")
}

/// spec: IMPORT-001.4
#[test]
fn image_number_is_the_trailing_digits() {
    let values = values_with_base("DSC_0042");

    assert_eq!(
        render("{number}.{ext}", &values, "DSC_0042.RAF").unwrap(),
        "0042.RAF"
    );
    assert_eq!(
        render("{number:2}.{ext}", &values, "DSC_0042.RAF").unwrap(),
        "42.RAF"
    );
    assert_eq!(
        render("{number:6}.{ext}", &values, "DSC_0042.RAF").unwrap(),
        "000042.RAF"
    );
}

/// spec: IMPORT-001.5
#[test]
fn file_without_image_number_cannot_be_named() {
    let template = NameTemplate::parse("{number}.{ext}").unwrap();

    let failure = template.render(&values_with_base("IMG"), "IMG.JPG", CompatibilityMode::On);
    let planned = template.render(
        &values_with_base("DSC_0042"),
        "DSC_0042.RAF",
        CompatibilityMode::On,
    );

    let error = failure.unwrap_err();
    assert_eq!(
        error,
        RenderError::NoImageNumber {
            token: "{number}".to_owned(),
            base_name: "IMG".to_owned(),
        }
    );
    assert!(error.to_string().contains("{number}"), "{error}");
    assert_eq!(planned.unwrap(), "0042.RAF");
}

/// spec: IMPORT-001.6
#[test]
fn import_time_is_the_value_given_for_the_job() {
    // The import time is fixed when the job is queued and passed in as a value, so
    // rendering after midnight still uses the queued day.
    let template = NameTemplate::parse("{imported:%Y-%m-%d}/{name}.{ext}").unwrap();
    let mut values = values_with_base("DSC_0042");
    values.import_time = time("2026-10-18 23:59:00");

    for file_name in ["DSC_0042.RAF", "DSC_0042.JPG"] {
        let path = template
            .render(&values, file_name, CompatibilityMode::On)
            .unwrap();
        assert!(path.starts_with("2026-10-18/"), "{path}");
    }
}

/// spec: IMPORT-001.7
#[test]
fn missing_capture_time_falls_back_to_modification_time() {
    let files = [FileTimes {
        role: AssetFileRole::Rendered,
        capture_time: None,
        modification_time: time("2026-03-01 08:00:00"),
    }];

    let date = asset_date(&files).unwrap();
    let values = values("scan", "2000-01-01 00:00:00");
    let values = AssetValues {
        capture_time: date.time,
        ..values
    };

    assert_eq!(date.source, DateSource::ModificationTime);
    assert_eq!(
        render("{date:%Y-%m-%d}/{name}.{ext}", &values, "scan.jpg").unwrap(),
        "2026-03-01/scan.jpg"
    );
}

/// spec: IMPORT-001.8
#[test]
fn files_of_one_asset_share_the_capture_time() {
    let files = [
        FileTimes {
            role: AssetFileRole::Rendered,
            capture_time: None,
            modification_time: time("2026-10-18 00:00:01"),
        },
        FileTimes {
            role: AssetFileRole::Raw,
            capture_time: Some(time("2026-10-17 23:59:59").into()),
            modification_time: time("2026-10-18 00:00:02"),
        },
    ];

    let date = asset_date(&files).unwrap();
    let values = AssetValues {
        capture_time: date.time,
        ..values_with_base("DSC_0042")
    };
    let template = "{date:%Y-%m-%d}/{name}.{ext}";

    assert_eq!(date.source, DateSource::CaptureTime);
    assert_eq!(
        render(template, &values, "DSC_0042.RAF").unwrap(),
        "2026-10-17/DSC_0042.RAF"
    );
    assert_eq!(
        render(template, &values, "DSC_0042.JPG").unwrap(),
        "2026-10-17/DSC_0042.JPG"
    );
}

/// spec: IMPORT-001.8
#[test]
fn capture_time_of_the_first_other_file_is_used_without_raw() {
    let first = time("2026-10-17 10:00:00");
    let files = [
        FileTimes {
            role: AssetFileRole::Rendered,
            capture_time: None,
            modification_time: time("2026-10-18 00:00:00"),
        },
        FileTimes {
            role: AssetFileRole::Rendered,
            capture_time: Some(first.into()),
            modification_time: time("2026-10-18 00:00:00"),
        },
        FileTimes {
            role: AssetFileRole::Rendered,
            capture_time: Some(time("2026-10-16 10:00:00").into()),
            modification_time: time("2026-10-18 00:00:00"),
        },
    ];

    let date = asset_date(&files).unwrap();

    assert_eq!((date.time, date.source), (first, DateSource::CaptureTime));
    assert!(asset_date(&[]).is_none());
}

/// spec: IMPORT-001.8
#[test]
fn modification_time_falls_back_in_the_same_order_as_the_capture_time() {
    // The RAW file's modification time wins even though it is later, showing that the
    // fallback follows the RAW-first order rather than the earliest modification time.
    let raw_modified = time("2026-10-19 00:00:00");
    let with_raw = [
        FileTimes {
            role: AssetFileRole::Rendered,
            capture_time: None,
            modification_time: time("2026-10-18 00:00:00"),
        },
        FileTimes {
            role: AssetFileRole::Raw,
            capture_time: None,
            modification_time: raw_modified,
        },
    ];
    let date = asset_date(&with_raw).unwrap();
    assert_eq!(
        (date.time, date.source),
        (raw_modified, DateSource::ModificationTime)
    );

    // Without a RAW file, the fallback is the first file in the given order, not the
    // one with the earliest modification time.
    let first_modified = time("2026-10-20 00:00:00");
    let without_raw = [
        FileTimes {
            role: AssetFileRole::Rendered,
            capture_time: None,
            modification_time: first_modified,
        },
        FileTimes {
            role: AssetFileRole::Rendered,
            capture_time: None,
            modification_time: time("2026-10-10 00:00:00"),
        },
    ];
    let date = asset_date(&without_raw).unwrap();
    assert_eq!(
        (date.time, date.source),
        (first_modified, DateSource::ModificationTime)
    );
}

/// spec: IMPORT-001.9
#[test]
fn sidecars_follow_their_file() {
    let template = NameTemplate::parse("{date:%Y%m%d}-{number}.{ext}").unwrap();
    let values = values("DSC_0042", "2026-10-17 14:05:09");

    let names: Vec<String> = ["DSC_0042.RAF", "DSC_0042.RAF.xmp", "DSC_0042.xmp"]
        .into_iter()
        .map(|file_name| {
            template
                .render(&values, file_name, CompatibilityMode::On)
                .unwrap()
        })
        .collect();

    assert_eq!(
        names,
        [
            "20261017-0042.RAF",
            "20261017-0042.RAF.xmp",
            "20261017-0042.xmp"
        ]
    );
}

/// spec: IMPORT-001.16
#[test]
fn compatibility_mode_replaces_invalid_characters() {
    let values = values("what?", "2026-10-17 14:05:09");

    let path = render("{date:%H:%M} {name}. /{name}.{ext}", &values, "what?.jpg");

    assert_eq!(path.unwrap(), "14_05 what_/what_.jpg");
}

/// spec: IMPORT-001.16
#[test]
fn compatibility_mode_replaces_control_characters() {
    let values = values("a\u{1}b\u{7f}<>\"\\|*", "2026-10-17 14:05:09");

    let name = render("{name}.{ext}", &values, "a\u{1}b\u{7f}<>\"\\|*.jpg");

    assert_eq!(name.unwrap(), "a_b_______.jpg");
}

/// spec: IMPORT-001.18
#[test]
fn rendered_segment_must_not_vanish() {
    let values = values("..", "2026-10-17 14:05:09");

    let path = render("{name}/x.{ext}", &values, "...jpg");

    assert_eq!(
        path,
        Err(RenderError::InvalidSegment {
            segment: String::new()
        })
    );
}

/// spec: IMPORT-001.17
#[test]
fn compatibility_mode_off_keeps_values() {
    let template = NameTemplate::parse("{date:%H:%M}/{name}.{ext}").unwrap();
    let values = values("what?", "2026-10-17 14:05:09");

    let path = template.render(&values, "what?.jpg", CompatibilityMode::Off);

    assert_eq!(path.unwrap(), "14:05/what?.jpg");
}

/// spec: IMPORT-001.17
#[test]
fn compatibility_mode_is_on_by_default() {
    assert_eq!(CompatibilityMode::default(), CompatibilityMode::On);
}
