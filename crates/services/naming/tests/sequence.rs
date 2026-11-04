// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rendering sequence numbers and checking their range (IMPORT-002).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use domain::LocalDateTime;
use naming::{AssetValues, CompatibilityMode, NameTemplate, SequenceKind};

fn values(number: u32) -> AssetValues {
    let time = LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap();
    AssetValues {
        base_name: "DSC_0042".to_owned(),
        capture_time: time,
        import_time: time,
        job_code: None,
        sequence_number: Some(number),
    }
}

fn render(template: &str, number: u32) -> String {
    NameTemplate::parse(template)
        .unwrap()
        .render(&values(number), "DSC_0042.RAF", CompatibilityMode::On)
        .unwrap()
}

/// spec: IMPORT-002.1
#[test]
fn digit_sequences_are_padded_with_zeros() {
    assert_eq!(
        render("{date:%Y%m%d}_{seq_today:3}.{ext}", 13),
        "20261017_013.RAF"
    );
    assert_eq!(render("{seq_stored:4}.{ext}", 251), "0251.RAF");
    assert_eq!(render("{seq_session:1}.{ext}", 3), "3.RAF");
    assert_eq!(render("{seq_session:6}.{ext}", 999_999), "999999.RAF");
}

/// spec: IMPORT-002.6
#[test]
fn letter_sequence_continues_after_z_with_aa_and_ab() {
    let names: Vec<String> = (26..=28)
        .map(|number| render("{seq_letters}.{ext}", number))
        .collect();

    assert_eq!(names, ["z.RAF", "aa.RAF", "ab.RAF"]);
}

/// spec: IMPORT-002.13
#[test]
fn letter_sequence_ends_at_zz() {
    assert_eq!(render("{seq_letters}.{ext}", 702), "zz.RAF");
}

/// spec: IMPORT-002.7
#[test]
fn overflow_names_the_template_and_suggests_one_more_digit() {
    let template = NameTemplate::parse("{seq_stored:2}.{ext}").unwrap();

    let overflow = template.check_sequence_range(98 + 5).unwrap_err();

    assert_eq!(overflow.template, "{seq_stored:2}.{ext}");
    assert_eq!(overflow.suggestion.as_deref(), Some("{seq_stored:3}.{ext}"));
    assert!(overflow.to_string().contains("{seq_stored:2}.{ext}"));
    assert!(overflow.to_string().contains("{seq_stored:3}.{ext}"));
}

/// spec: IMPORT-002.7
#[test]
fn a_range_that_fits_is_accepted() {
    let template = NameTemplate::parse("{seq_stored:2}.{ext}").unwrap();

    assert!(template.check_sequence_range(99).is_ok());
}

/// spec: IMPORT-002.13
#[test]
fn letters_beyond_zz_suggest_a_session_sequence_with_enough_digits() {
    let template = NameTemplate::parse("{seq_letters}.{ext}").unwrap();

    assert!(template.check_sequence_range(702).is_ok());
    let overflow = template.check_sequence_range(703).unwrap_err();

    assert_eq!(overflow.template, "{seq_letters}.{ext}");
    assert_eq!(
        overflow.suggestion.as_deref(),
        Some("{seq_session:3}.{ext}")
    );
}

#[test]
fn the_template_tells_its_sequence_kind() {
    let kind = |text| NameTemplate::parse(text).unwrap().sequence_kind();

    assert_eq!(
        kind("{seq_today:3}.{ext}"),
        Some(SequenceKind::ImportedToday)
    );
    assert_eq!(kind("{seq_stored:3}.{ext}"), Some(SequenceKind::Stored));
    assert_eq!(kind("{seq_session:3}.{ext}"), Some(SequenceKind::Session));
    assert_eq!(kind("{seq_letters}.{ext}"), Some(SequenceKind::Letters));
    assert_eq!(kind("{{seq_today:3}}.{ext}"), None);
    assert!(SequenceKind::Stored.is_persisted());
    assert!(!SequenceKind::Letters.is_persisted());
}
