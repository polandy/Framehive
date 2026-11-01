// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Validation of job codes (IMPORT-006).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use import::{JobCode, JobCodeError, MissingJobCode, require_job_code};
use naming::NameTemplate;

/// spec: IMPORT-006.1
#[test]
fn surrounding_whitespace_is_removed() {
    let job_code = JobCode::parse("  Smith wedding \n").unwrap();

    assert_eq!(job_code.as_str(), "Smith wedding");
    assert_eq!(job_code.to_string(), "Smith wedding");
}

/// spec: IMPORT-006.2
#[test]
fn sixty_four_characters_are_accepted() {
    let text = "ü".repeat(64);

    let job_code = JobCode::parse(&text).unwrap();

    assert_eq!(job_code.as_str(), text);
}

/// spec: IMPORT-006.2
#[test]
fn sixty_five_characters_are_rejected() {
    let error = JobCode::parse(&"a".repeat(65)).unwrap_err();

    assert_eq!(error, JobCodeError::TooLong { characters: 65 });
    assert!(error.to_string().contains("64 characters"), "{error}");
}

/// spec: IMPORT-006.3
#[test]
fn empty_text_is_rejected() {
    for text in ["", "   "] {
        let error = JobCode::parse(text).unwrap_err();

        assert_eq!(error, JobCodeError::Empty, "{text:?}");
        assert!(error.to_string().contains("empty"), "{error}");
    }
}

/// spec: IMPORT-006.4
#[test]
fn slash_is_rejected() {
    let error = JobCode::parse("Smith/Jones").unwrap_err();

    assert_eq!(error, JobCodeError::ContainsSlash);
    assert!(error.to_string().contains('/'), "{error}");
}

/// spec: IMPORT-006.5
#[test]
fn control_characters_are_rejected() {
    for text in ["Smith\tJones", "Smith\u{85}Jones", "Smith\u{7f}Jones"] {
        let error = JobCode::parse(text).unwrap_err();

        assert_eq!(error, JobCodeError::ContainsControlCharacter, "{text:?}");
        assert!(error.to_string().contains("control character"), "{error}");
    }
}

/// spec: IMPORT-006.6
#[test]
fn a_template_with_job_needs_a_job_code() {
    let template = NameTemplate::parse("{job}/{name}.{ext}").unwrap();
    let job_code = JobCode::parse("Smith wedding").unwrap();

    let error = require_job_code(&template, None).unwrap_err();

    assert_eq!(error, MissingJobCode);
    assert!(error.to_string().contains("{job}"), "{error}");
    assert_eq!(require_job_code(&template, Some(&job_code)), Ok(()));
}

/// spec: IMPORT-006.7
#[test]
fn a_template_without_job_needs_no_job_code() {
    let template = NameTemplate::parse("{name}.{ext}").unwrap();

    assert_eq!(require_job_code(&template, None), Ok(()));
}
