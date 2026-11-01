// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rendering the `{job}` token (IMPORT-001, IMPORT-006).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use domain::LocalDateTime;
use naming::{AssetValues, CompatibilityMode, NameTemplate, RenderError};

fn values(job_code: Option<&str>) -> AssetValues {
    AssetValues {
        base_name: "DSC_0042".to_owned(),
        capture_time: LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap(),
        import_time: LocalDateTime::new(2026, 10, 19, 9, 0, 0).unwrap(),
        job_code: job_code.map(str::to_owned),
    }
}

fn render(template: &str, values: &AssetValues) -> Result<String, RenderError> {
    NameTemplate::parse(template)
        .unwrap()
        .render(values, "DSC_0042.RAF", CompatibilityMode::On)
}

/// spec: IMPORT-001.19
#[test]
fn job_code_is_inserted_and_made_compatible() {
    let path = render(
        "{date:%Y}/{job}/{name}.{ext}",
        &values(Some("Smith: wedding")),
    );

    assert_eq!(path.unwrap(), "2026/Smith_ wedding/DSC_0042.RAF");
}

/// spec: IMPORT-006.6
#[test]
fn job_token_without_job_code_cannot_be_named() {
    let error = render("{job}/{name}.{ext}", &values(None));

    assert_eq!(error, Err(RenderError::NoJobCode));
}
