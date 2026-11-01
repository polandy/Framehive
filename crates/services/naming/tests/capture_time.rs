// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Names from a capture time that carries sub-seconds and an offset (CORE-005).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use domain::{AssetFileRole, CaptureTime, LocalDateTime, UtcOffset};
use naming::{AssetValues, CompatibilityMode, FileTimes, NameTemplate, asset_date};

/// spec: CORE-005.2
#[test]
fn sub_seconds_and_offset_do_not_change_the_rendered_time() {
    let capture_time = CaptureTime::new(LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap())
        .with_subsec_nanoseconds(420_000_000)
        .unwrap()
        .with_offset(UtcOffset::from_seconds(2 * 3_600).unwrap());
    let files = [FileTimes {
        role: AssetFileRole::Rendered,
        capture_time: Some(capture_time),
        modification_time: LocalDateTime::new(2026, 10, 18, 9, 0, 0).unwrap(),
    }];

    let date = asset_date(&files).unwrap();
    let values = AssetValues {
        base_name: "DSC_0042".to_owned(),
        capture_time: date.time,
        import_time: LocalDateTime::new(2026, 10, 19, 9, 0, 0).unwrap(),
        job_code: None,
    };
    let template = NameTemplate::parse("{date:%H%M%S}.{ext}").unwrap();

    assert_eq!(
        template
            .render(&values, "DSC_0042.JPG", CompatibilityMode::On)
            .unwrap(),
        "140509.JPG"
    );
}
