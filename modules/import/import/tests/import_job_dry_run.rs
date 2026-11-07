// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A dry run of several import jobs counts their numbers and target names in memory
//! (IMPORT-016): the preview matches a real run, and nothing persisted is written.

#![allow(clippy::unwrap_used, clippy::panic, reason = "tests")]

mod job_support;
mod run_support;

use std::path::Path;
use std::sync::Arc;

use files::testing::MemoryFreeSpace;
use import::{DryRunMemory, ImportJob, ImportJobReports};
use job_support::shared::{Shared, job_settings};
use job_support::{Steps, summary};
use jobs::JobQueue;
use run_support::target;

const STORED: &str = "{seq_stored:3}.{ext}";

/// A dry-run job from `source` to `destination` with `template`, planned with the
/// counters of `memory` and given it, as an application does (IMPORT-016).
fn dry_run_job(
    shared: &Shared,
    memory: &Arc<DryRunMemory>,
    (source, destination, template): (&str, &str, &str),
) -> (ImportJob, ImportJobReports) {
    let mut ports = shared.ports();
    ports.counters = Arc::clone(memory) as _;
    let mut settings = job_settings(source, destination, template);
    settings.dry_run = true;
    let scan = import::scan_source(
        &shared.file_system,
        Path::new(source),
        Path::new(destination),
    )
    .unwrap();
    let plan = ports
        .import_run()
        .plan(&settings.settings(), &scan)
        .unwrap();
    let (job, reports) = ImportJob::new(ports, settings);
    (
        job.with_plan(plan).with_dry_run_memory(Arc::clone(memory)),
        reports,
    )
}

/// The targets the job of `reports` would copy to, once it ended.
fn would_copy(reports: ImportJobReports) -> Vec<String> {
    let summary = summary(reports.follow(&mut Steps::default()));
    assert!(summary.dry_run);
    summary
        .files
        .iter()
        .filter_map(|report| target(report).map(str::to_owned))
        .collect()
}

/// spec: IMPORT-016.1
#[test]
fn dry_run_jobs_continue_each_others_numbers() {
    let shared = Shared::new(MemoryFreeSpace::unlimited());
    shared.source("/media/a", &["a1.jpg", "a2.jpg", "a3.jpg"]);
    shared.source("/media/b", &["b1.jpg", "b2.jpg"]);
    shared.stored_at(100);
    let memory = Arc::new(DryRunMemory::new(Arc::clone(&shared.counters) as _));
    let (mut queue, _updates) = JobQueue::start();

    let (three, three_reports) = dry_run_job(&shared, &memory, ("/media/a", "/library", STORED));
    three.enqueue(&mut queue).unwrap();
    let (two, two_reports) = dry_run_job(&shared, &memory, ("/media/b", "/library", STORED));
    two.enqueue(&mut queue).unwrap();

    assert_eq!(would_copy(three_reports), ["101.jpg", "102.jpg", "103.jpg"]);
    assert_eq!(would_copy(two_reports), ["104.jpg", "105.jpg"]);
    assert_eq!(shared.stored_last(), 100);
}

/// spec: IMPORT-016.2
#[test]
fn a_target_name_of_an_ended_dry_run_job_stays_taken() {
    let shared = Shared::new(MemoryFreeSpace::unlimited());
    shared.source("/media/a", &["a.jpg"]);
    shared.source("/media/b", &["a.jpg"]);
    let memory = Arc::new(DryRunMemory::new(Arc::clone(&shared.counters) as _));
    let (mut queue, _updates) = JobQueue::start();
    let names = ("/library", "{name}.{ext}");

    let (first, first_reports) = dry_run_job(&shared, &memory, ("/media/a", names.0, names.1));
    first.enqueue(&mut queue).unwrap();
    let first_targets = would_copy(first_reports);
    let (second, second_reports) = dry_run_job(&shared, &memory, ("/media/b", names.0, names.1));
    second.enqueue(&mut queue).unwrap();

    assert_eq!(first_targets, ["a.jpg"]);
    assert_eq!(would_copy(second_reports), ["a-1.jpg"]);
    let library = Path::new("/library");
    assert!(
        shared
            .file_system
            .paths()
            .iter()
            .all(|path| !path.starts_with(library))
    );
}
