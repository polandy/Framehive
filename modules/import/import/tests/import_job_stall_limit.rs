// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The stall limit of a source (IMPORT-022). The jobs read their own source through a
//! `ManualTimer`; a helper thread lets the limit pass once a read hangs, so nothing
//! sleeps.

#![allow(clippy::unwrap_used, clippy::panic, reason = "tests")]

mod job_support;
mod run_support;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use files::EntryKind;
use files::ReadOnlyFileSystem;
use files::testing::{Fault, Hang, ManualTimer, MemoryFileSystem, SlowReads};
use history::testing::MemoryImportHistory;
use import::{
    DEFAULT_SOURCE_STALL_LIMIT, FileReport, ImportJob, ImportJobEnd, ImportJobReports, ImportPorts,
    ImportSummary, RunError,
};
use job_support::{Steps, copied, ports, run_in_queue, settings, summary};
use jobs::JobState;
use run_support::{DESTINATION, MODIFIED, SOURCE, TEMPLATE, add, failure, source_with, target};

const PHOTOS: [&str; 3] = [
    "DCIM/100_FUJI/DSCF0001.RAF",
    "DCIM/100_FUJI/DSCF0002.RAF",
    "DCIM/100_FUJI/DSCF0003.RAF",
];

/// The ports of a job that reads an own source with `timer`, and its destination.
fn ports_with(timer: &Arc<ManualTimer>) -> (ImportPorts, MemoryFileSystem) {
    let destination = MemoryFileSystem::new();
    let history = Arc::new(MemoryImportHistory::new());
    let ports = ImportPorts {
        timer: Arc::clone(timer) as _,
        ..ports(&destination, &history)
    };
    (ports, destination)
}

/// A job that imports `source` by the plan made before its faults are injected.
fn planned_job(
    ports: ImportPorts,
    source: &MemoryFileSystem,
    limit: Duration,
) -> (ImportJob, ImportJobReports) {
    let settings = settings(TEMPLATE).with_source_stall_limit(limit);
    let scan = import::scan_source(source, Path::new(SOURCE), Path::new(DESTINATION)).unwrap();
    let plan = ports
        .import_run()
        .with_source(source)
        .plan(&settings.settings(), &scan)
        .unwrap();
    let (job, reports) = ImportJob::new(ports, settings);
    let job = job
        .with_source(Box::new(source.clone()), PathBuf::from("camera:X-T5"))
        .with_plan(plan);
    (job, reports)
}

/// Lets `limit` pass on `timer` once a call hangs on `hang` and the job waits for it,
/// and returns how many calls the source had made by then.
fn pass_the_limit_once_it_hangs(
    hang: &Hang,
    timer: &Arc<ManualTimer>,
    source: &MemoryFileSystem,
    limit: Duration,
) -> thread::JoinHandle<usize> {
    let (hang, timer, source) = (hang.clone(), Arc::clone(timer), source.clone());
    thread::spawn(move || {
        hang.wait_until_begun();
        while timer.waits() == 0 {
            thread::yield_now();
        }
        let calls = source.operations();
        timer.advance(limit);
        calls
    })
}

/// Imports a source with [`PHOTOS`] whose second photo hangs, with `limit`.
struct HangingImport {
    summary: ImportSummary,
    destination: MemoryFileSystem,
    timer: Arc<ManualTimer>,
    calls_when_it_hung: usize,
    calls_at_the_end: usize,
    state: JobState,
}

fn import_with_a_hang(limit: Duration) -> HangingImport {
    let timer = Arc::new(ManualTimer::new());
    let (ports, destination) = ports_with(&timer);
    let hang = Hang::new();
    let source = source_with(&PHOTOS);
    let job = planned_job(ports, &source, limit);
    source.inject_fault(
        &Path::new(SOURCE).join(PHOTOS[1]),
        Fault::Hangs(hang.clone()),
    );
    let helper = pass_the_limit_once_it_hangs(&hang, &timer, &source, limit);

    let (end, state) = run_in_queue(job, &mut Steps::default());

    let calls_at_the_end = source.operations();
    hang.release();
    HangingImport {
        summary: summary(end),
        destination,
        timer,
        calls_when_it_hung: helper.join().unwrap(),
        calls_at_the_end,
        state,
    }
}

fn report<'s>(summary: &'s ImportSummary, photo: &str) -> &'s FileReport {
    summary
        .files
        .iter()
        .find(|report| report.path == Path::new(photo))
        .unwrap()
}

fn failure_message(summary: &ImportSummary, photo: &str) -> String {
    failure(report(summary, photo)).unwrap().to_string()
}

/// The paths of the files at the destination.
fn files_at(destination: &MemoryFileSystem) -> Vec<String> {
    let mut files = Vec::new();
    collect_files(destination, Path::new(DESTINATION), &mut files);
    files
}

fn collect_files(file_system: &MemoryFileSystem, folder: &Path, files: &mut Vec<String>) {
    let Ok(entries) = file_system.list_folder(folder) else {
        return;
    };
    for entry in entries {
        let path = folder.join(&entry.name);
        match entry.kind {
            EntryKind::Folder => collect_files(file_system, &path, files),
            _ => files.push(path.display().to_string()),
        }
    }
}

/// spec: IMPORT-022.1
#[test]
fn a_hanging_read_fails_the_file_at_the_limit() {
    let import = import_with_a_hang(DEFAULT_SOURCE_STALL_LIMIT);

    assert!(target(report(&import.summary, PHOTOS[0])).is_some());
    assert_eq!(
        failure_message(&import.summary, PHOTOS[1]),
        "cannot read the source: /media/card/DCIM/100_FUJI/DSCF0002.RAF: \
         the source did not answer for 30 seconds"
    );
    assert_eq!(import.timer.time_since_start(), Duration::from_secs(30));
    let files = files_at(&import.destination);
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(files[0].ends_with("DSCF0001.RAF"), "{files:?}");
    assert_eq!(import.state, JobState::Finished);
}

/// spec: IMPORT-022.2
#[test]
fn the_remaining_files_of_a_stopped_source_fail_at_once() {
    let import = import_with_a_hang(DEFAULT_SOURCE_STALL_LIMIT);

    assert_eq!(
        failure_message(&import.summary, PHOTOS[2]),
        "cannot read the source: /media/card/DCIM/100_FUJI/DSCF0003.RAF: \
         the source stopped answering"
    );
    assert_eq!(import.calls_at_the_end, import.calls_when_it_hung);
    assert_eq!(import.timer.waits(), 0);
    assert_eq!(import.timer.time_since_start(), Duration::from_secs(30));
}

/// spec: IMPORT-022.3
#[test]
fn a_slow_source_that_keeps_answering_is_not_failed() {
    let timer = Arc::new(ManualTimer::new());
    let (ports, destination) = ports_with(&timer);
    let source = source_with(&[]);
    let content = vec![7; 10 * 64 * 1024 + 1];
    add(&source, PHOTOS[0], &content, MODIFIED);
    let job = planned_job(ports, &source, DEFAULT_SOURCE_STALL_LIMIT);
    let slow = SlowReads::new(Arc::clone(&timer), Duration::from_secs(20));
    source.inject_fault(&Path::new(SOURCE).join(PHOTOS[0]), Fault::SlowReads(slow));

    let (end, state) = run_in_queue(job, &mut Steps::default());

    assert_eq!(copied(&summary(end)), 1);
    assert!(timer.time_since_start() >= Duration::from_mins(3));
    assert_eq!(files_at(&destination).len(), 1);
    assert_eq!(state, JobState::Finished);
}

/// spec: IMPORT-022.4
#[test]
fn other_sources_go_on() {
    let timer = Arc::new(ManualTimer::new());
    let (ports, destination) = ports_with(&timer);
    let hang = Hang::new();
    let first = source_with(&PHOTOS);
    let second = source_with(&["DCIM/101_FUJI/DSCF0101.RAF"]);
    let first_job = planned_job(ports.clone(), &first, DEFAULT_SOURCE_STALL_LIMIT);
    let second_job = planned_job(ports, &second, DEFAULT_SOURCE_STALL_LIMIT);
    first.inject_fault(
        &Path::new(SOURCE).join(PHOTOS[1]),
        Fault::Hangs(hang.clone()),
    );
    let helper = pass_the_limit_once_it_hangs(&hang, &timer, &first, DEFAULT_SOURCE_STALL_LIMIT);

    let (first_end, _) = run_in_queue(first_job, &mut Steps::default());
    let (second_end, second_state) = run_in_queue(second_job, &mut Steps::default());

    hang.release();
    helper.join().unwrap();
    let first_summary = summary(first_end);
    assert!(failure_message(&first_summary, PHOTOS[1]).ends_with("did not answer for 30 seconds"));
    assert!(failure_message(&first_summary, PHOTOS[2]).ends_with("stopped answering"));
    assert_eq!(copied(&summary(second_end)), 1);
    assert!(
        files_at(&destination)
            .iter()
            .any(|path| path.ends_with("DSCF0101.RAF"))
    );
    assert_eq!(second_state, JobState::Finished);
}

/// spec: IMPORT-022.5
#[test]
fn a_hanging_scan_fails_the_source() {
    let timer = Arc::new(ManualTimer::new());
    let (ports, destination) = ports_with(&timer);
    let hang = Hang::new();
    let source = source_with(&PHOTOS);
    source.inject_fault(
        &Path::new(SOURCE).join("DCIM/100_FUJI"),
        Fault::Hangs(hang.clone()),
    );
    let helper = pass_the_limit_once_it_hangs(&hang, &timer, &source, DEFAULT_SOURCE_STALL_LIMIT);
    let (job, reports) = ImportJob::new(ports, settings(TEMPLATE));
    let job = job.with_source(Box::new(source.clone()), PathBuf::from("camera:X-T5"));

    let (end, state) = run_in_queue((job, reports), &mut Steps::default());

    hang.release();
    helper.join().unwrap();
    let error = end.and_then(|end: ImportJobEnd| end.result.err()).unwrap();
    assert!(matches!(error, RunError::Scan(_)), "{error:?}");
    assert_eq!(
        error.to_string(),
        "/media/card/DCIM/100_FUJI: the source did not answer for 30 seconds"
    );
    assert!(files_at(&destination).is_empty());
    assert!(matches!(state, JobState::Failed(_)));
}

/// spec: IMPORT-022.6
#[test]
fn the_limit_of_the_settings_is_used() {
    let import = import_with_a_hang(Duration::from_secs(5));

    assert_eq!(
        failure_message(&import.summary, PHOTOS[1]),
        "cannot read the source: /media/card/DCIM/100_FUJI/DSCF0002.RAF: \
         the source did not answer for 5 seconds"
    );
    assert_eq!(import.timer.time_since_start(), Duration::from_secs(5));
}
