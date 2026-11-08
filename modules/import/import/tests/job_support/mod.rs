// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import jobs, the enqueuers and the queue shared by the import job tests
//! (IMPORT-015).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]
#![allow(dead_code, reason = "each test binary uses only some of the helpers")]

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{FnvHasher, MemoryFileSystem, MemoryFreeSpace};
use history::testing::MemoryImportHistory;
use import::{
    ConflictPolicy, FileOutcome, FileReport, ImportJob, ImportJobEnd, ImportJobSettings,
    ImportPorts, ImportSettings, ImportSummary, ProgressSink, ReservedTargetNames, SessionSequence,
    SourceScan,
};
use jobs::{JobId, JobQueue, JobState, JobUpdate};
use naming::testing::MemorySequenceCounters;
use naming::{CompatibilityMode, NameTemplate};

pub(crate) mod shared;

use crate::run_support::{DESTINATION, NOW, SOURCE, TEMPLATE};

/// The ports of a job on `file_system`, with a history the test can look into.
pub(crate) fn ports(
    file_system: &MemoryFileSystem,
    history: &Arc<MemoryImportHistory>,
) -> ImportPorts {
    let clock = ManualClock::new(Timestamp::from_unix_seconds(NOW), UtcOffset::UTC);
    ImportPorts {
        file_system: Arc::new(file_system.clone()),
        hasher: Arc::new(FnvHasher),
        clock: Arc::new(clock),
        counters: Arc::new(MemorySequenceCounters::new()),
        session: Arc::new(SessionSequence::new()),
        history: Arc::clone(history) as _,
        free_space: Arc::new(MemoryFreeSpace::unlimited()),
        reserved_names: Arc::new(ReservedTargetNames::new()),
    }
}

pub(crate) fn settings(template: &str) -> ImportJobSettings {
    let template = NameTemplate::parse(template).unwrap();
    ImportJobSettings::of(&ImportSettings {
        source: Path::new(SOURCE),
        destination: Path::new(DESTINATION),
        backup_destinations: &[],
        video_destination: None,
        video_backup_destinations: &[],
        template: &template,
        video_template: None,
        job_code: None,
        mode: CompatibilityMode::On,
        policy: ConflictPolicy::AddSuffix,
        dry_run: false,
    })
}

/// A job that imports `file_system`'s source with [`TEMPLATE`].
pub(crate) fn job(file_system: &MemoryFileSystem) -> (ImportJob, import::ImportJobReports) {
    let history = Arc::new(MemoryImportHistory::new());
    ImportJob::new(ports(file_system, &history), settings(TEMPLATE))
}

/// Every step the enqueuer saw, as text.
#[derive(Default)]
pub(crate) struct Steps(pub(crate) Vec<String>);

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
pub(crate) fn end_state(updates: &Receiver<JobUpdate>, id: JobId) -> JobState {
    updates
        .iter()
        .find(|update| update.job == id && update.state.has_ended())
        .unwrap()
        .state
}

/// Runs `job` in a new queue, follows its reports with `progress`, and returns its end
/// report and its end state.
pub(crate) fn run_in_queue(
    job: (ImportJob, import::ImportJobReports),
    progress: &mut dyn ProgressSink,
) -> (Option<ImportJobEnd>, JobState) {
    let (mut queue, updates) = JobQueue::start();
    let (job, reports) = job;
    let id = queue.enqueue(Box::new(job));
    let end = reports.follow(progress);
    (end, end_state(&updates, id))
}

pub(crate) fn summary(end: Option<ImportJobEnd>) -> ImportSummary {
    end.unwrap().result.unwrap()
}

pub(crate) fn copied(summary: &ImportSummary) -> usize {
    summary
        .files
        .iter()
        .filter(|report| matches!(report.outcome, FileOutcome::Copied { .. }))
        .count()
}

/// Answers "stop" at the first question.
pub(crate) struct StopsAtOnce;

impl ProgressSink for StopsAtOnce {
    fn scan_finished(&mut self, _scan: &SourceScan) {}
    fn file_started(&mut self, _path: &Path) {}
    fn file_finished(&mut self, _report: &FileReport) {}
    fn should_continue(&mut self) -> bool {
        false
    }
}
