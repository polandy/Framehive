// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A job, its id, its state, and how it learns that it should stop (CORE-007).

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Background work for the [`JobQueue`](crate::JobQueue).
///
/// A job owns everything it needs, because it moves to the worker thread (ADR 0035
/// §4). What it reports beyond its state, it sends to its enqueuer itself.
pub trait Job: Send + 'static {
    /// Does the work on the worker thread. The job asks `control` whether to go on at
    /// the points where it can stop safely, and returns how it ended.
    fn run(self: Box<Self>, control: &JobControl) -> JobEnd;
}

/// How a job ended, as it tells the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEnd {
    /// It ran to its end.
    Finished,
    /// It was asked to stop and ended early.
    Stopped,
    /// It ended with an error; the error's text.
    Failed(String),
}

/// Tells a running job whether it was asked to stop (ADR 0035 §5).
#[derive(Debug, Clone, Default)]
pub struct JobControl {
    stop: Arc<AtomicBool>,
}

impl JobControl {
    /// A control that has not been asked to stop.
    #[must_use]
    pub fn new() -> Self {
        JobControl::default()
    }

    /// Whether the job goes on: `false` once it was asked to stop.
    #[must_use]
    pub fn should_continue(&self) -> bool {
        !self.stop.load(Ordering::SeqCst)
    }

    /// Asks the job to stop at the next point where it asks.
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// The id of a job in its queue: 1, 2, 3, … in the order the jobs were enqueued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(u64);

impl JobId {
    /// The job id with this number.
    #[must_use]
    pub fn new(number: u64) -> Self {
        JobId(number)
    }

    /// The number of this job id.
    #[must_use]
    pub fn number(self) -> u64 {
        self.0
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "job {}", self.0)
    }
}

/// Where a job stands (CORE-007, "Job states").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    /// Enqueued, waiting for the jobs before it.
    Queued,
    /// The worker runs it.
    Running,
    /// It ran to its end.
    Finished,
    /// It was asked to stop while it ran, and ended early.
    Stopped,
    /// It ended with an error, or stopped unexpectedly; the text says why.
    Failed(String),
    /// It was removed before it ran, and never ran.
    Removed,
}

impl JobState {
    /// Whether the job has ended: no state follows this one.
    #[must_use]
    pub fn has_ended(&self) -> bool {
        !matches!(self, JobState::Queued | JobState::Running)
    }
}

impl From<JobEnd> for JobState {
    fn from(end: JobEnd) -> Self {
        match end {
            JobEnd::Finished => JobState::Finished,
            JobEnd::Stopped => JobState::Stopped,
            JobEnd::Failed(reason) => JobState::Failed(reason),
        }
    }
}

/// A change of a job's state, as the queue reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobUpdate {
    /// The job.
    pub job: JobId,
    /// Its new state.
    pub state: JobState,
}
