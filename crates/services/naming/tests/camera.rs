// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rendering the `{camera}` token (IMPORT-021).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use domain::LocalDateTime;
use naming::{AssetValues, CompatibilityMode, NameTemplate, TemplateErrorKind};

fn render(camera_model: Option<&str>, mode: CompatibilityMode) -> String {
    let values = AssetValues {
        base_name: "DSC_0042".to_owned(),
        capture_time: LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap(),
        import_time: LocalDateTime::new(2026, 10, 19, 9, 0, 0).unwrap(),
        job_code: None,
        sequence_number: None,
        camera_model: camera_model.map(str::to_owned),
    };
    let template = NameTemplate::parse("{camera}/{name}.{ext}").unwrap();
    let path = template.render(&values, "DSC_0042.RAF", mode).unwrap();
    path.strip_suffix("/DSC_0042.RAF").unwrap().to_owned()
}

fn folder(camera_model: Option<&str>) -> String {
    render(camera_model, CompatibilityMode::On)
}

/// spec: IMPORT-021.1
#[test]
fn the_model_names_the_folder() {
    assert_eq!(folder(Some("X-T5")), "X-T5");
}

/// spec: IMPORT-021.2
#[test]
fn a_missing_model_gives_the_fallback() {
    assert_eq!(folder(None), "Unknown camera");
    assert_eq!(folder(Some("")), "Unknown camera");
}

/// spec: IMPORT-021.3
#[test]
fn characters_invalid_in_file_names_become_underscores_in_both_modes() {
    for mode in [CompatibilityMode::On, CompatibilityMode::Off] {
        assert_eq!(render(Some("A/B:C*\"D\"?"), mode), "A_B_C__D__");
        assert_eq!(render(Some("a\\b<c>|d\u{1}e"), mode), "a_b_c__d_e");
    }
}

/// spec: IMPORT-021.4
#[test]
fn blanks_are_collapsed_and_edges_are_trimmed() {
    assert_eq!(folder(Some("  Canon   EOS  7D. . ")), "Canon EOS 7D");
    for only_blanks_and_dots in [" ", ".", "..", ". . ", "\t"] {
        assert_eq!(folder(Some(only_blanks_and_dots)), "Unknown camera");
    }
}

/// spec: IMPORT-021.5
#[test]
fn a_very_long_model_is_cut_at_64_characters() {
    assert_eq!(folder(Some(&"x".repeat(200))), "x".repeat(64));
    let multi_byte = "é".repeat(200);
    assert_eq!(folder(Some(&multi_byte)), "é".repeat(64));
    let ends_in_blank = format!("{} {}", "y".repeat(63), "z".repeat(10));
    assert_eq!(folder(Some(&ends_in_blank)), "y".repeat(63));
}

/// spec: IMPORT-021.7
#[test]
fn the_token_takes_no_option() {
    let error = NameTemplate::parse("{camera:upper}/{name}.{ext}").unwrap_err();

    assert_eq!(
        error.kind,
        TemplateErrorKind::UnknownOption {
            token: "{camera:upper}".to_owned()
        }
    );
}
