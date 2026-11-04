// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A template with a persisted sequence counter (IMPORT-002) in the guided import.

use std::path::Path;

use naming::StoredCounters;

use super::support::{Harness, OCTOBER_17, OCTOBER_19, PICTURES, PRESET_1, Step};

/// A harness with `template`, the stored counter at `stored_last` and the card
/// `EOS_DIGITAL` with `DSCF0001.RAF` and `DSCF0002.RAF`.
fn with_card(template: &str, stored_last: u32) -> Harness {
    let harness = Harness::new();
    harness.remember(Some(PICTURES), Some(template), &[]);
    harness.counters.set(StoredCounters {
        stored_last,
        ..StoredCounters::default()
    });
    harness.insert("EOS_DIGITAL");
    harness.add_file(
        "EOS_DIGITAL",
        "DCIM/100_FUJI/DSCF0001.RAF",
        b"first",
        OCTOBER_17,
    );
    harness.add_file(
        "EOS_DIGITAL",
        "DCIM/100_FUJI/DSCF0002.RAF",
        b"second",
        OCTOBER_19,
    );
    harness
}

/// spec: CLI-003.13
#[test]
fn the_summary_warns_and_enter_reserves_the_numbers() {
    let harness = with_card("{seq_stored:4}.{ext}", 250);

    let transcript = harness.run(vec![Step::Line("")]);

    assert!(
        transcript.output.contains(
            "\x20 named     {seq_stored:4}.{ext}\n\
             Files already imported are not recognized, because the template has a persisted sequence counter.\n"
        ),
        "{}",
        transcript.output
    );
    let library = Path::new(PICTURES);
    let first = harness.memory.content(&library.join("0251.RAF"));
    let second = harness.memory.content(&library.join("0252.RAF"));
    assert_eq!(first.as_deref(), Some(&b"first"[..]));
    assert_eq!(second.as_deref(), Some(&b"second"[..]));
    assert_eq!(harness.counters.stored().stored_last, 252);
}

/// spec: CLI-003.14
#[test]
fn numbers_that_do_not_fit_lead_to_the_template_step() {
    let harness = with_card("{seq_stored:2}.{ext}", 98);

    let transcript = harness.run(vec![Step::Line("1"), Step::Line("y")]);

    let output = &transcript.output;
    let reason = output
        .find("The template cannot number EOS_DIGITAL: ")
        .unwrap_or_else(|| panic!("{output}"));
    let after = &output[reason..];
    let line = after.lines().next().unwrap();
    assert!(line.contains("{seq_stored:3}.{ext}"), "{output}");
    let template_step = after.find("How should the photos be named?").unwrap();
    let summary = after
        .find(&format!("\x20 named     {PRESET_1}\n"))
        .unwrap_or_else(|| panic!("{output}"));
    assert!(template_step < summary, "{output}");
    assert_eq!(harness.counters.stored().stored_last, 98);
}
