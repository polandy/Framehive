// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The template step and the job code prompt.

use std::path::Path;

use super::support::{Harness, OCTOBER_17, PICTURES, PRESET_1, PRESET_2, PRESET_3, Step};

const PRESETS: &str = "How should the photos be named?\n\
    \x20 1  By day:              {date:%Y}/{date:%Y-%m-%d}/{name}.{ext}\n\
    \x20 2  By day and job code: {date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}\n\
    \x20 3  By month, renamed:   {date:%Y}/{date:%Y-%m}/{date:%Y%m%d}_{number:4}.{ext}\n\
    Choose 1–3 or type a template [1]: ";

/// A harness with a remembered destination and `template`, and the card
/// `EOS_DIGITAL` with `DSCF0001.RAF`.
fn with_card(template: Option<&str>, job_codes: &[&str]) -> Harness {
    let harness = Harness::new();
    harness.remember(Some(PICTURES), template, job_codes);
    harness.insert("EOS_DIGITAL");
    harness.add_file(
        "EOS_DIGITAL",
        "DCIM/100_FUJI/DSCF0001.RAF",
        b"raf",
        OCTOBER_17,
    );
    harness
}

/// spec: CLI-002.7
#[test]
fn a_preset_is_previewed_and_remembered() {
    let harness = with_card(None, &[]);

    let transcript = harness.run(vec![Step::Line("3"), Step::Line("y")]);

    assert!(
        transcript.output.contains(&format!(
            "Reading EOS_DIGITAL…\n{PRESETS}3\n\
         Preview with the first 1 files of EOS_DIGITAL:\n\
         \x20 DCIM/100_FUJI/DSCF0001.RAF -> 2026/2026-10/20261017_0001.RAF\n\
         Use this template? [Y/n]: y\n"
        )),
        "{}",
        transcript.output
    );
    assert!(
        harness
            .state_text()
            .contains(&format!("template={PRESET_3}\n"))
    );
}

/// spec: CLI-002.8
#[test]
fn an_invalid_template_is_shown_with_a_caret() {
    let harness = with_card(None, &[]);

    let transcript = harness.run(vec![Step::Line("{date:%Y}/{camera}/{name}.{ext}")]);

    let output = &transcript.output;
    let error =
        "{date:%Y}/{camera}/{name}.{ext}\n  {date:%Y}/{camera}/{name}.{ext}\n            ^ ";
    let shown = output.find(error).unwrap();
    assert!(output[shown..].contains(PRESETS), "{output}");
    let caret_line = output[shown..].lines().nth(2).unwrap();
    assert!(caret_line.len() > "            ^ ".len(), "{caret_line}");
}

/// spec: CLI-002.9
#[test]
fn a_rejected_preview_starts_the_step_again() {
    let harness = with_card(None, &[]);

    let transcript = harness.run(vec![Step::Line("1"), Step::Line("n")]);

    assert!(
        transcript
            .output
            .contains(&format!("Use this template? [Y/n]: n\n{PRESETS}\n"))
    );
    assert!(!harness.state_text().contains("template="));
}

/// spec: CLI-002.10
#[test]
fn the_preview_shows_the_first_five_files() {
    let harness = with_card(None, &[]);
    harness.add_file("EOS_DIGITAL", "DCIM/100_FUJI/ABC.RAF", b"abc", OCTOBER_17);
    for number in 2..=6 {
        let path = format!("DCIM/100_FUJI/DSCF000{number}.RAF");
        harness.add_file("EOS_DIGITAL", &path, path.as_bytes(), OCTOBER_17);
    }

    let transcript = harness.run(vec![Step::Line("3")]);

    assert!(transcript.output.contains(
        "Preview with the first 5 files of EOS_DIGITAL:\n\
         \x20 DCIM/100_FUJI/ABC.RAF -> cannot be named: {number:4} needs an image number, but \"ABC\" does not end in a digit\n\
         \x20 DCIM/100_FUJI/DSCF0001.RAF -> 2026/2026-10/20261017_0001.RAF\n\
         \x20 DCIM/100_FUJI/DSCF0002.RAF -> 2026/2026-10/20261017_0002.RAF\n\
         \x20 DCIM/100_FUJI/DSCF0003.RAF -> 2026/2026-10/20261017_0003.RAF\n\
         \x20 DCIM/100_FUJI/DSCF0004.RAF -> 2026/2026-10/20261017_0004.RAF\n\
         Use this template?"
    ), "{}", transcript.output);
}

/// spec: CLI-002.11
#[test]
fn a_template_without_job_asks_for_no_job_code() {
    let harness = with_card(Some(PRESET_1), &["Harbour"]);

    let transcript = harness.run(vec![]);

    let output = &transcript.output;
    assert!(!output.contains("Job code"), "{output}");
    assert!(!output.contains("job code"), "{output}");
    assert!(
        output.ends_with(
            "\n  named     {date:%Y}/{date:%Y-%m-%d}/{name}.{ext}\n\
         [Enter] import  [l] list  [t] template  [d] destination  [q] quit: \n"
        ),
        "{output}"
    );
}

/// spec: CLI-002.12
#[test]
fn a_remembered_job_code_is_chosen_by_number() {
    let harness = with_card(Some(PRESET_2), &["Smith wedding", "Harbour"]);

    let transcript = harness.run(vec![Step::Line("2"), Step::Line(""), Step::Line("n")]);

    assert!(
        transcript.output.contains(
            "Job code:\n  1  Smith wedding\n  2  Harbour\nChoose 1–2 or type a job code [1]: 2\n"
        ),
        "{}",
        transcript.output
    );
    let copied = Path::new(PICTURES).join("2026/2026-10-17_Harbour/DSCF0001.RAF");
    assert_eq!(harness.memory.content(&copied), Some(b"raf".to_vec()));
    assert!(
        harness
            .state_text()
            .ends_with("job_code=Harbour\njob_code=Smith wedding\n")
    );
}

/// spec: CLI-002.13
#[test]
fn a_new_job_code_is_checked() {
    let harness = with_card(Some(PRESET_2), &[]);

    let transcript = harness.run(vec![
        Step::Line(""),
        Step::Line("Smith/Jones"),
        Step::Line("Smith wedding"),
    ]);

    assert!(
        transcript.output.contains(
            "Job code: \n\
         A job code is needed, because the template has {job}.\n\
         Job code: Smith/Jones\n\
         Invalid job code: a job code must not contain \"/\".\n\
         Job code: Smith wedding\n"
        ),
        "{}",
        transcript.output
    );
    assert!(transcript.output.contains("  job code  Smith wedding\n"));
}
