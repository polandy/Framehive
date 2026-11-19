// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A `/` or NUL in a value never adds a folder to a rendered path (IMPORT-001).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use domain::LocalDateTime;
use naming::{AssetValues, CompatibilityMode, NameTemplate, RenderError};

fn with_job_code(job_code: &str) -> AssetValues {
    let time = LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap();
    AssetValues {
        base_name: "DSC_0042".to_owned(),
        capture_time: time,
        import_time: time,
        job_code: Some(job_code.to_owned()),
        sequence_number: None,
        camera_model: None,
    }
}

fn render(job_code: &str, mode: CompatibilityMode) -> Result<String, RenderError> {
    NameTemplate::parse("{job}/{name}.{ext}").unwrap().render(
        &with_job_code(job_code),
        "DSC_0042.RAF",
        mode,
    )
}

/// spec: IMPORT-001.16
#[test]
fn compatibility_mode_replaces_a_slash_in_a_value() {
    let path = render("../a/b", CompatibilityMode::On);

    assert_eq!(path.unwrap(), ".._a_b/DSC_0042.RAF");
}

/// spec: IMPORT-001.18
#[test]
fn without_compatibility_mode_a_slash_or_nul_in_a_value_cannot_be_named() {
    for job_code in ["../a", "a\u{0}b"] {
        let path = render(job_code, CompatibilityMode::Off);

        assert_eq!(
            path,
            Err(RenderError::InvalidSegment {
                segment: job_code.to_owned()
            })
        );
    }
}
