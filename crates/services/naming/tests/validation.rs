// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Validation of name templates when they are saved (IMPORT-001).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test helpers may panic on bad fixtures"
)]

use naming::{NameTemplate, TemplateError, TemplateErrorKind};

fn rejection(template: &str) -> TemplateError {
    NameTemplate::parse(template).expect_err("the template must be rejected")
}

/// spec: IMPORT-001.10
#[test]
fn unknown_token_is_rejected_with_its_column() {
    let error = rejection("{date:%Y}/{camera}.{ext}");

    assert_eq!(error.column, 11);
    assert_eq!(
        error.kind,
        TemplateErrorKind::UnknownToken {
            token: "{camera}".to_owned()
        }
    );
    let message = error.to_string();
    assert!(message.contains("{camera}"), "{message}");
    assert!(message.contains("column 11"), "{message}");
}

/// spec: IMPORT-001.10
#[test]
fn unknown_token_option_is_rejected() {
    for template in ["{name:title}.{ext}", "{number:7}.{ext}", "{number:0}.{ext}"] {
        let error = rejection(template);
        assert!(
            matches!(error.kind, TemplateErrorKind::UnknownOption { .. }),
            "{template}: {error}"
        );
        assert_eq!(error.column, 1, "{template}");
    }
}

/// spec: IMPORT-001.10
#[test]
fn date_token_without_format_is_rejected() {
    let error = rejection("{date}/{name}.{ext}");

    assert_eq!(
        error.kind,
        TemplateErrorKind::MissingOption {
            token: "{date}".to_owned()
        }
    );
}

/// spec: IMPORT-001.11
#[test]
fn unknown_date_code_is_rejected() {
    let error = rejection("{date:%Y-%B}/{name}.{ext}");

    assert_eq!(
        error.kind,
        TemplateErrorKind::UnknownDateCode {
            code: "%B".to_owned()
        }
    );
    assert_eq!(error.column, 10);
    assert!(error.to_string().contains("%B"));
}

/// spec: IMPORT-001.11
#[test]
fn date_format_with_slash_is_rejected() {
    let error = rejection("{date:%Y/%m}/{name}.{ext}");

    assert_eq!(error.kind, TemplateErrorKind::SlashInDateFormat);
    assert_eq!(error.column, 9);
}

/// spec: IMPORT-001.12
#[test]
fn unbalanced_closing_brace_is_rejected() {
    let error = rejection("photos}/{name}.{ext}");

    assert_eq!(error.kind, TemplateErrorKind::UnbalancedBrace);
    assert_eq!(error.column, 7);
    assert!(error.to_string().contains("column 7"));
}

/// spec: IMPORT-001.12
#[test]
fn unclosed_opening_brace_is_rejected() {
    assert_eq!(rejection("{name.{ext}").column, 7);
    assert_eq!(rejection("a/{name").column, 3);
}

/// spec: IMPORT-001.13
#[test]
fn second_sequence_token_is_rejected() {
    let error = rejection("{seq_today:3}-{seq_session:2}.{ext}");

    assert_eq!(
        error.kind,
        TemplateErrorKind::SecondSequenceToken {
            token: "{seq_session:2}".to_owned()
        }
    );
    assert_eq!(error.column, 15);
    assert!(error.to_string().contains("{seq_session:2}"));
}

/// spec: IMPORT-001.13
#[test]
fn one_sequence_token_is_accepted() {
    for template in [
        "{seq_today:3}.{ext}",
        "{seq_stored:6}.{ext}",
        "{seq_session:1}.{ext}",
        "{seq_letters}.{ext}",
    ] {
        assert!(NameTemplate::parse(template).is_ok(), "{template}");
    }
}

/// spec: IMPORT-001.14
#[test]
fn last_segment_must_end_with_extension() {
    for template in [
        "{date:%Y}/{name}",
        "{name}{ext}",
        "{ext}.{name}",
        "x/{name}.{ext}/y",
    ] {
        let error = rejection(template);
        assert_eq!(
            error.kind,
            TemplateErrorKind::MissingExtension,
            "{template}"
        );
        assert!(error.to_string().contains(".{ext}"), "{template}: {error}");
    }
}

/// spec: IMPORT-001.14
#[test]
fn extension_with_an_option_is_accepted() {
    assert!(NameTemplate::parse("{name}.{ext:lower}").is_ok());
    assert!(NameTemplate::parse("{name}.{ext:upper}").is_ok());
}

/// spec: IMPORT-001.15
#[test]
fn leading_slash_is_rejected() {
    let error = rejection("/{name}.{ext}");

    assert_eq!(error.kind, TemplateErrorKind::LeadingSlash);
    assert_eq!(error.column, 1);
}

/// spec: IMPORT-001.15
#[test]
fn empty_segment_is_rejected() {
    let error = rejection("a//{name}.{ext}");

    assert_eq!(error.kind, TemplateErrorKind::EmptySegment);
    assert_eq!(error.column, 3);
}

/// spec: IMPORT-001.15
#[test]
fn dot_segments_are_rejected() {
    let error = rejection("../{name}.{ext}");
    assert_eq!(
        error.kind,
        TemplateErrorKind::DotSegment {
            segment: "..".to_owned()
        }
    );
    assert_eq!(error.column, 1);
    assert!(error.to_string().contains("\"..\""));

    let error = rejection("a/./{name}.{ext}");
    assert_eq!(error.column, 3);
}

/// spec: IMPORT-001.20
#[test]
fn job_token_takes_no_option() {
    let error = rejection("{job:upper}/{name}.{ext}");

    assert_eq!(
        error.kind,
        TemplateErrorKind::UnknownOption {
            token: "{job:upper}".to_owned()
        }
    );
    assert!(error.to_string().contains("{job:upper}"), "{error}");
}

/// spec: IMPORT-006.6
#[test]
fn template_tells_whether_it_uses_the_job_code() {
    let uses = |text| NameTemplate::parse(text).unwrap().uses_job_code();

    assert!(uses("{date:%Y}/{job}-{name}.{ext}"));
    assert!(!uses("{{job}}/{name}.{ext}"));
    assert!(!uses("{name}.{ext}"));
}

/// spec: IMPORT-001.1
#[test]
fn template_keeps_its_text() {
    let text = "{date:%Y}/{date:%Y-%m-%d}/{date:%Y%m%d}-{name}.{ext}";

    assert_eq!(NameTemplate::parse(text).unwrap().as_str(), text);
}
