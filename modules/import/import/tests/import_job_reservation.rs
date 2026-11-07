// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What an import job reserves when it is enqueued: its sequence numbers and its target
//! names (IMPORT-002, IMPORT-015, ADR 0036). A gate job that names the same resources
//! keeps the import jobs waiting until the test opens it, so nothing sleeps.

#![allow(clippy::unwrap_used, clippy::panic, reason = "tests")]

mod job_support;
mod run_support;

use std::sync::mpsc;

use files::testing::MemoryFreeSpace;
use import::{ImportJobReports, RunError};
use job_support::shared::{GateOn, Shared};
use job_support::{Steps, summary};
use jobs::{Job, JobQueue};
use run_support::target;

const STORED: &str = "{seq_stored:3}.{ext}";

/// The targets of the files the job of `reports` copied.
fn copied(reports: ImportJobReports) -> Vec<String> {
    let summary = summary(reports.follow(&mut Steps::default()));
    summary
        .files
        .iter()
        .filter_map(|report| target(report).map(str::to_owned))
        .collect()
}

/// spec: IMPORT-002.16
#[test]
fn numbers_are_reserved_when_an_import_job_is_enqueued() {
    let shared = Shared::new(MemoryFreeSpace::unlimited());
    shared.source("/media/a", &["a1.jpg", "a2.jpg", "a3.jpg"]);
    shared.source("/media/b", &["b1.jpg", "b2.jpg"]);
    shared.stored_at(100);
    let (three, three_reports) = shared.planned_job("/media/a", "/library", STORED);
    let (two, two_reports) = shared.planned_job("/media/b", "/library", STORED);
    let (open, gate) = mpsc::channel();
    let (mut queue, _updates) = JobQueue::start();
    queue.enqueue(Box::new(GateOn {
        resources: three.resources(),
        gate,
    }));

    three.enqueue(&mut queue).unwrap();
    two.enqueue(&mut queue).unwrap();
    let stored_before_either_ran = shared.stored_last();
    open.send(()).unwrap();

    assert_eq!(stored_before_either_ran, 105);
    assert_eq!(copied(three_reports), ["101.jpg", "102.jpg", "103.jpg"]);
    assert_eq!(copied(two_reports), ["104.jpg", "105.jpg"]);
}

/// spec: IMPORT-002.17
#[test]
fn a_removed_import_job_leaves_a_gap() {
    let shared = Shared::new(MemoryFreeSpace::unlimited());
    shared.source("/media/a", &["a1.jpg", "a2.jpg", "a3.jpg"]);
    shared.source("/media/b", &["b1.jpg"]);
    shared.stored_at(100);
    let (removed, _removed_reports) = shared.planned_job("/media/a", "/library", STORED);
    let (open, gate) = mpsc::channel();
    let (mut queue, _updates) = JobQueue::start();
    queue.enqueue(Box::new(GateOn {
        resources: removed.resources(),
        gate,
    }));
    let removed = removed.enqueue(&mut queue).unwrap();

    queue.remove(removed).unwrap();
    let (other, other_reports) = shared.planned_job("/media/b", "/library", STORED);
    other.enqueue(&mut queue).unwrap();
    open.send(()).unwrap();

    assert_eq!(copied(other_reports), ["104.jpg"]);
}

/// spec: IMPORT-015.14
#[test]
fn a_reserved_target_name_counts_as_taken() {
    // No file system can be told, so each job names its folders and the two run in
    // parallel.
    let shared = Shared::new(MemoryFreeSpace::new());
    shared.source("/media/a", &["a.jpg"]);
    shared.source("/media/b", &["a.jpg"]);
    let (waiting, _held) = shared.planned_job("/media/a", "/library", "sub/{name}.{ext}");
    let (second, second_reports) = shared.planned_job("/media/b", "/library/sub", "{name}.{ext}");
    let (mut queue, _updates) = JobQueue::start();

    waiting
        .asking_before_each_file()
        .enqueue(&mut queue)
        .unwrap();
    second.enqueue(&mut queue).unwrap();

    assert_eq!(copied(second_reports), ["a-1.jpg"]);
}

/// spec: IMPORT-015.15
#[test]
fn a_job_that_cannot_reserve_is_not_enqueued() {
    let shared = Shared::new(MemoryFreeSpace::unlimited());
    shared.source("/media/a", &["a1.jpg", "a2.jpg", "a3.jpg"]);
    shared.source("/media/b", &["b1.jpg", "b2.jpg"]);
    shared.stored_at(95);
    let (first, _first_reports) =
        shared.planned_job("/media/a", "/library", "{seq_stored:2}.{ext}");
    let (second, second_reports) =
        shared.planned_job("/media/b", "/library/b", "{seq_stored:2}.{ext}");
    let (mut queue, updates) = JobQueue::start();
    let first = first.enqueue(&mut queue).unwrap();

    let error = second.enqueue(&mut queue).unwrap_err();
    queue.shut_down();

    assert!(matches!(error, RunError::Sequence(_)));
    assert!(updates.iter().all(|update| update.job == first));
    assert_eq!(shared.stored_last(), 98);
    assert!(second_reports.follow(&mut Steps::default()).is_none());
}
