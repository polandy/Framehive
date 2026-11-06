// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! An import as a job in the job queue (IMPORT-015). The test follows the job's
//! reports on its own thread; the job waits for its answers where a test needs a fixed
//! order, so nothing sleeps.

#![allow(clippy::unwrap_used, clippy::panic, reason = "tests")]

mod run_support;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{Fault, FnvHasher, MemoryFileSystem, MemoryFreeSpace};
use history::testing::MemoryImportHistory;
use import::{
    ConflictPolicy, FileOutcome, FileReport, ImportJob, ImportJobEnd, ImportJobSettings,
    ImportPorts, ImportSettings, ImportSummary, ProgressSink, RunError, SessionSequence,
    SourceScan,
};
use jobs::{Job, JobControl, JobEnd, JobId, JobQueue, JobState, JobUpdate};
use naming::testing::MemorySequenceCounters;
use naming::{CompatibilityMode, NameTemplate};
use run_support::{DESTINATION, NOW, SOURCE, Setup, TEMPLATE, source_with, target};

/// The ports of a job on `file_system`, with a history the test can look into.
fn ports(file_system: &MemoryFileSystem, history: &Arc<MemoryImportHistory>) -> ImportPorts {
    let clock = ManualClock::new(Timestamp::from_unix_seconds(NOW), UtcOffset::UTC);
    ImportPorts {
        file_system: Arc::new(file_system.clone()),
        hasher: Arc::new(FnvHasher),
        clock: Arc::new(clock),
        counters: Arc::new(MemorySequenceCounters::new()),
        session: Arc::new(SessionSequence::new()),
        history: Arc::clone(history) as _,
        free_space: Arc::new(MemoryFreeSpace::unlimited()),
    }
}

fn settings(template: &str) -> ImportJobSettings {
    let template = NameTemplate::parse(template).unwrap();
    ImportJobSettings::of(&ImportSettings {
        source: Path::new(SOURCE),
        destination: Path::new(DESTINATION),
        backup_destinations: &[],
        video_destination: None,
        video_backup_destinations: &[],
        template: &template,
        job_code: None,
        mode: CompatibilityMode::On,
        policy: ConflictPolicy::AddSuffix,
        dry_run: false,
    })
}

/// A job that imports `file_system`'s source with [`TEMPLATE`].
fn job(file_system: &MemoryFileSystem) -> (ImportJob, import::ImportJobReports) {
    let history = Arc::new(MemoryImportHistory::new());
    ImportJob::new(ports(file_system, &history), settings(TEMPLATE))
}

/// Every step the enqueuer saw, as text.
#[derive(Default)]
struct Steps(Vec<String>);

impl ProgressSink for Steps {
    fn scan_finished(&mut self, scan: &SourceScan) {
        self.0.push(format!("scan {}", scan.files.len()));
    }

    fn file_started(&mut self, path: &Path) {
        self.0.push(format!("started {}", path.display()));
    }

    fn file_finished(&mut self, report: &FileReport) {
        self.0.push(format!("finished {}", report.path.display()));
    }
}

/// How the job `id` ended, from the queue's updates.
fn end_state(updates: &Receiver<JobUpdate>, id: JobId) -> JobState {
    updates
        .iter()
        .find(|update| update.job == id && update.state.has_ended())
        .unwrap()
        .state
}

/// Runs `job` in a new queue, follows its reports with `progress`, and returns its end
/// report and its end state.
fn run_in_queue(
    job: (ImportJob, import::ImportJobReports),
    progress: &mut dyn ProgressSink,
) -> (Option<ImportJobEnd>, JobState) {
    let (mut queue, updates) = JobQueue::start();
    let (job, reports) = job;
    let id = queue.enqueue(Box::new(job));
    let end = reports.follow(progress);
    (end, end_state(&updates, id))
}

fn summary(end: Option<ImportJobEnd>) -> ImportSummary {
    end.unwrap().result.unwrap()
}

fn copied(summary: &ImportSummary) -> usize {
    summary
        .files
        .iter()
        .filter(|report| matches!(report.outcome, FileOutcome::Copied { .. }))
        .count()
}

/// spec: IMPORT-015.1
#[test]
fn a_job_imports_like_a_run() {
    let names = ["DCIM/a.jpg", "DCIM/b.jpg", "DCIM/c.jpg"];
    let by_run = source_with(&names);
    let by_job = source_with(&names);

    let run = Setup::default().on(&by_run, &mut Steps::default());
    let (end, state) = run_in_queue(job(&by_job), &mut Steps::default());

    let job = summary(end);
    let targets = |summary: &ImportSummary| -> Vec<Option<String>> {
        summary
            .files
            .iter()
            .map(|report| target(report).map(str::to_owned))
            .collect()
    };
    assert_eq!(targets(&job), targets(&run));
    assert_eq!(copied(&job), 3);
    assert_eq!(copied(&job), copied(&run));
    assert_eq!(by_job.paths(), by_run.paths());
    assert_eq!(state, JobState::Finished);
}

/// spec: IMPORT-015.2
#[test]
fn the_enqueuer_receives_the_reports_in_order() {
    let file_system = source_with(&["DCIM/a.jpg", "DCIM/b.jpg"]);
    let mut steps = Steps::default();

    let (end, state) = run_in_queue(job(&file_system), &mut steps);

    assert_eq!(
        steps.0,
        [
            "scan 2",
            "started DCIM/a.jpg",
            "finished DCIM/a.jpg",
            "started DCIM/b.jpg",
            "finished DCIM/b.jpg",
        ]
    );
    assert_eq!(summary(end).files.len(), 2);
    assert_eq!(state, JobState::Finished);
}

/// spec: IMPORT-015.3
#[test]
fn an_import_that_cannot_start_fails() {
    let file_system = source_with(&["DCIM/a.jpg"]);
    let history = Arc::new(MemoryImportHistory::new());
    let job = ImportJob::new(
        ports(&file_system, &history),
        settings("{job}/{name}.{ext}"),
    );
    let mut steps = Steps::default();

    let (end, state) = run_in_queue(job, &mut steps);

    let error = end.unwrap().result.unwrap_err();
    assert!(matches!(error, RunError::MissingJobCode(_)), "{error:?}");
    assert_eq!(state, JobState::Failed(error.to_string()));
    assert!(steps.0.is_empty(), "{:?}", steps.0);
    assert!(
        !file_system
            .paths()
            .iter()
            .any(|path| path.starts_with(DESTINATION))
    );
}

/// spec: IMPORT-015.4
#[test]
fn failed_files_still_finish_the_job() {
    let file_system = source_with(&["DCIM/a.jpg", "DCIM/b.jpg"]);
    file_system.inject_fault(
        &Path::new(SOURCE).join("DCIM/b.jpg"),
        Fault::ReadFails { after_bytes: 0 },
    );

    let (end, state) = run_in_queue(job(&file_system), &mut Steps::default());

    let summary = summary(end);
    assert_eq!(copied(&summary), 1);
    let failed = summary
        .files
        .iter()
        .filter(|report| matches!(report.outcome, FileOutcome::Failed(_)))
        .count();
    assert_eq!(failed, 1);
    assert_eq!(state, JobState::Finished);
}

/// Stops the job it follows once the first file started.
struct StopsAtFirstFile<'q> {
    queue: &'q JobQueue,
    id: JobId,
}

impl ProgressSink for StopsAtFirstFile<'_> {
    fn scan_finished(&mut self, _scan: &SourceScan) {}

    fn file_started(&mut self, _path: &Path) {
        let _ = self.queue.stop(self.id);
    }

    fn file_finished(&mut self, _report: &FileReport) {}
}

/// spec: IMPORT-015.5
#[test]
fn stopping_an_import_job_finishes_the_file_being_copied() {
    let file_system = source_with(&["DCIM/a.jpg", "DCIM/b.jpg", "DCIM/c.jpg"]);
    let history = Arc::new(MemoryImportHistory::new());
    let (job, reports) = ImportJob::new(ports(&file_system, &history), settings(TEMPLATE));
    // The job asks before each file, so the stop is seen before the second one.
    let job = job.asking_before_each_file();
    let (mut queue, updates) = JobQueue::start();
    let id = queue.enqueue(Box::new(job));

    let end = reports.follow(&mut StopsAtFirstFile { queue: &queue, id });

    let summary = summary(end);
    assert_eq!(summary.files.len(), 1);
    assert_eq!(copied(&summary), 1);
    assert!(summary.stopped);
    let runs = history.runs();
    assert_eq!(runs.len(), 1);
    assert!(runs[0].finished_at.is_some(), "{runs:?}");
    assert_eq!(history.copies().len(), 1);
    assert_eq!(end_state(&updates, id), JobState::Stopped);
}

/// A job that waits until the test opens its gate.
struct Gate(Receiver<()>);

impl Job for Gate {
    fn run(self: Box<Self>, _control: &JobControl) -> JobEnd {
        let _ = self.0.recv();
        JobEnd::Finished
    }
}

/// spec: IMPORT-015.6
#[test]
fn a_removed_import_job_does_not_touch_the_source() {
    let file_system = source_with(&[]);
    let camera = source_with(&["DCIM/a.jpg"]);
    let history = Arc::new(MemoryImportHistory::new());
    let (job, reports) = ImportJob::new(ports(&file_system, &history), settings(TEMPLATE));
    let job = job.with_source(Box::new(camera.clone()), PathBuf::from("camera:X-T5"));
    let (open, gate): (Sender<()>, Receiver<()>) = mpsc::channel();
    let (mut queue, updates) = JobQueue::start();
    queue.enqueue(Box::new(Gate(gate)));
    let id = queue.enqueue(Box::new(job));

    queue.remove(id).unwrap();
    open.send(()).unwrap();
    let mut steps = Steps::default();
    let end = reports.follow(&mut steps);

    assert!(end.is_none());
    assert!(steps.0.is_empty());
    assert_eq!(camera.operations(), 0);
    assert_eq!(end_state(&updates, id), JobState::Removed);
}

/// The answers of a job that ended, with the source it handed back listed.
fn handed_back(end: Option<ImportJobEnd>) -> Vec<PathBuf> {
    let source = end.unwrap().source.unwrap();
    source
        .list_folder(Path::new(SOURCE))
        .unwrap()
        .into_iter()
        .map(|entry| entry.name.into())
        .collect()
}

/// A job from the camera `camera` with the template `template`.
fn camera_job(camera: &MemoryFileSystem, template: &str) -> (ImportJob, import::ImportJobReports) {
    let history = Arc::new(MemoryImportHistory::new());
    let (job, reports) = ImportJob::new(
        ports(&MemoryFileSystem::new(), &history),
        settings(template),
    );
    let job = job.with_source(Box::new(camera.clone()), PathBuf::from("camera:X-T5"));
    (job, reports)
}

/// Answers "stop" at the first question.
struct StopsAtOnce;

impl ProgressSink for StopsAtOnce {
    fn scan_finished(&mut self, _scan: &SourceScan) {}
    fn file_started(&mut self, _path: &Path) {}
    fn file_finished(&mut self, _report: &FileReport) {}
    fn should_continue(&mut self) -> bool {
        false
    }
}

/// spec: IMPORT-015.7
#[test]
fn an_opened_camera_comes_back() {
    let camera = source_with(&["DCIM/a.jpg"]);

    let (finished, finished_state) =
        run_in_queue(camera_job(&camera, TEMPLATE), &mut Steps::default());
    let (job, reports) = camera_job(&camera, TEMPLATE);
    let (stopped, stopped_state) =
        run_in_queue((job.asking_before_each_file(), reports), &mut StopsAtOnce);
    let (failed, failed_state) = run_in_queue(
        camera_job(&camera, "{job}/{name}.{ext}"),
        &mut Steps::default(),
    );

    assert_eq!(finished_state, JobState::Finished);
    assert_eq!(stopped_state, JobState::Stopped);
    assert!(matches!(failed_state, JobState::Failed(_)));
    for end in [finished, stopped, failed] {
        assert_eq!(handed_back(end), [PathBuf::from("DCIM")]);
    }
}

/// spec: IMPORT-015.8
#[test]
fn a_job_imports_a_plan_made_on_the_callers_thread() {
    let file_system = source_with(&["DCIM/a.jpg", "DCIM/b.jpg"]);
    let history = Arc::new(MemoryImportHistory::new());
    let ports = ports(&file_system, &history);
    let settings = settings(TEMPLATE);
    let scan =
        import::scan_source(&file_system, Path::new(SOURCE), Path::new(DESTINATION)).unwrap();
    let plan = ports
        .import_run()
        .plan(&settings.settings(), &scan)
        .unwrap();
    let (job, reports) = ImportJob::new(ports, settings);
    let mut steps = Steps::default();

    let (end, state) = run_in_queue((job.with_plan(plan), reports), &mut steps);

    let summary = summary(end);
    assert_eq!(copied(&summary), 2);
    assert_eq!(
        steps.0.first().map(String::as_str),
        Some("started DCIM/a.jpg")
    );
    assert_eq!(history.runs().len(), 1);
    assert_eq!(state, JobState::Finished);
}

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
