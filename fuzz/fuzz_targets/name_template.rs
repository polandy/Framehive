// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Fuzz target `name_template`: any text goes to `NameTemplate::parse`, and an accepted
//! template is rendered for `DSC_0042.RAF` in both compatibility modes (IMPORT-001,
//! ADR 0037).
//!
//! A panic, a column outside the template, or a rendered path that leaves the
//! destination is a finding (IMPORT-001.21).

#![no_main]

use domain::LocalDateTime;
use libfuzzer_sys::fuzz_target;
use naming::{AssetValues, CompatibilityMode, NameTemplate};

fuzz_target!(|content: &[u8]| {
    let Ok(text) = std::str::from_utf8(content) else {
        return;
    };
    let template = match NameTemplate::parse(text) {
        Ok(template) => template,
        Err(error) => {
            assert!((1..=text.chars().count() + 1).contains(&error.column));
            return;
        }
    };
    let values = AssetValues {
        base_name: "DSC_0042".to_owned(),
        capture_time: LocalDateTime::new(2026, 10, 17, 14, 5, 9).expect("a valid time"),
        import_time: LocalDateTime::new(2026, 10, 19, 9, 0, 0).expect("a valid time"),
        job_code: Some("Smith: wedding".to_owned()),
        sequence_number: Some(7),
        camera_model: Some("X-T5".to_owned()),
    };
    for mode in [CompatibilityMode::On, CompatibilityMode::Off] {
        if let Ok(path) = template.render(&values, "DSC_0042.RAF", mode) {
            assert!(
                path.split('/').all(|segment| !matches!(segment, "" | "." | "..")),
                "{text:?} gives {path:?}"
            );
        }
    }
});
