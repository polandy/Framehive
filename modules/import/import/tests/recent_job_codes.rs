// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The list of recent job codes (IMPORT-006).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use import::{JobCode, MAX_RECENT_JOB_CODES, RecentJobCodes};

fn job_code(text: &str) -> JobCode {
    JobCode::parse(text).unwrap()
}

fn texts(recent: &RecentJobCodes) -> Vec<&str> {
    recent.job_codes().iter().map(JobCode::as_str).collect()
}

/// spec: IMPORT-006.9
#[test]
fn using_a_job_code_puts_it_first() {
    let mut recent = RecentJobCodes::from_stored(["Harbour", "Smith wedding"]);

    recent.mark_used(job_code("Smith wedding"));
    recent.mark_used(job_code("Alps"));

    assert_eq!(texts(&recent), ["Alps", "Smith wedding", "Harbour"]);
}

/// spec: IMPORT-006.10
#[test]
fn at_most_ten_recent_job_codes_are_kept() {
    let stored: Vec<String> = (1..=10)
        .rev()
        .map(|number| format!("Job {number}"))
        .collect();
    let mut recent = RecentJobCodes::from_stored(&stored);
    assert_eq!(texts(&recent).last(), Some(&"Job 1"));

    recent.mark_used(job_code("Alps"));

    assert_eq!(MAX_RECENT_JOB_CODES, 10);
    assert_eq!(recent.job_codes().len(), 10);
    assert_eq!(texts(&recent)[0], "Alps");
    assert!(!texts(&recent).contains(&"Job 1"));
}

/// spec: IMPORT-006.11
#[test]
fn the_list_is_built_from_valid_stored_job_codes_once_each() {
    let recent = RecentJobCodes::from_stored(["Harbour", "smith", "Smith", "Harbour", "a/b"]);

    assert_eq!(texts(&recent), ["Harbour", "smith", "Smith"]);
}

#[test]
fn more_than_ten_stored_job_codes_keep_the_first_ten() {
    let stored: Vec<String> = (1..=12).map(|number| format!("Job {number}")).collect();

    let recent = RecentJobCodes::from_stored(&stored);

    assert_eq!(recent.job_codes().len(), 10);
    assert_eq!(texts(&recent).last(), Some(&"Job 10"));
}
