// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! An import job that asks its enqueuer before each file (IMPORT-015.9, IMPORT-015.10).

#![allow(clippy::unwrap_used, clippy::panic, reason = "tests")]

mod job_support;
mod run_support;

use job_support::{Steps, StopsAtOnce, end_state, job, run_in_queue, summary};
use jobs::{JobQueue, JobState};
use run_support::source_with;

/// spec: IMPORT-015.9
#[test]
fn an_asking_job_stops_when_its_enqueuer_says_so() {
    let file_system = source_with(&["DCIM/a.jpg", "DCIM/b.jpg"]);
    let (job, reports) = job(&file_system);

    let (end, state) = run_in_queue((job.asking_before_each_file(), reports), &mut StopsAtOnce);

    let summary = summary(end);
    assert!(summary.stopped);
    assert!(summary.files.is_empty());
    assert_eq!(state, JobState::Stopped);
}

/// spec: IMPORT-015.10
#[test]
fn a_job_waiting_for_an_answer_stops_when_asked() {
    let file_system = source_with(&["DCIM/a.jpg"]);
    let (job, reports) = job(&file_system);
    let (mut queue, updates) = JobQueue::start();
    let id = queue.enqueue(Box::new(job.asking_before_each_file()));

    // The reports are held but not followed: shutting down must not wait for them.
    queue.shut_down();
    let mut steps = Steps::default();
    let end = reports.follow(&mut steps);

    assert!(!steps.0.iter().any(|step| step.starts_with("started")));
    assert!(summary(end).stopped);
    assert_eq!(end_state(&updates, id), JobState::Stopped);
}
