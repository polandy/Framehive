// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A job, its id, its resources, its state, and how it learns that it should stop
//! (CORE-007).

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// Background work for the [`JobQueue`](crate::JobQueue).
///
/// A job owns everything it needs, because it moves to a worker thread (ADR 0035
/// §4). What it reports beyond its state, it sends to its enqueuer itself.
pub trait Job: Send + 'static {
    /// Does the work on the worker thread. The job asks `control` whether to go on at
    /// the points where it can stop safely, and returns how it ended.
    fn run(self: Box<Self>, control: &JobControl) -> JobEnd;

    /// The resources the job uses, read once when it is enqueued. Two jobs that name a
    /// common resource run one after another (CORE-007, ADR 0036 §2). By default a job
    /// names none and runs alongside every other job.
    fn resources(&self) -> Vec<ResourceKey> {
        Vec::new()
    }
}

/// An opaque name of something a job uses, such as a device or a file system
/// (glossary: Resource key). The queue only compares keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceKey(String);

impl ResourceKey {
    /// The resource named `name`.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        ResourceKey(name.into())
    }

    /// The name of the resource.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ResourceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
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

/// Tells a running job whether it was asked to stop (ADR 0035 §5), and tells a job that
/// waits for something when it is asked (ADR 0036 §7).
#[derive(Debug, Clone, Default)]
pub struct JobControl {
    stop: Arc<StopRequest>,
}

/// A notice that a job asked for, called once when the job is asked to stop.
type StopNotice = Box<dyn FnOnce() + Send>;

/// Whether a job was asked to stop, and who waits to be told.
#[derive(Default)]
struct StopRequest {
    asked: AtomicBool,
    /// The notices not yet given; set and read together with `asked`, under this lock.
    notices: Mutex<Vec<StopNotice>>,
}

impl fmt::Debug for StopRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StopRequest")
            .field("asked", &self.asked.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl StopRequest {
    fn notices(&self) -> std::sync::MutexGuard<'_, Vec<StopNotice>> {
        // A notice runs after the lock is released, so a panicking notice cannot
        // leave the list half-changed.
        self.notices.lock().unwrap_or_else(PoisonError::into_inner)
    }
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
        !self.stop.asked.load(Ordering::SeqCst)
    }

    /// Asks the job to stop at the next point where it asks, and gives every notice
    /// that waits for it.
    pub fn request_stop(&self) {
        let notices = {
            let mut notices = self.stop.notices();
            self.stop.asked.store(true, Ordering::SeqCst);
            std::mem::take(&mut *notices)
        };
        for notice in notices {
            notice();
        }
    }

    /// Calls `notice` once when the job is asked to stop, or at once if it already was,
    /// so that a job waiting for something, such as an answer, need not wait on
    /// (CORE-007.16). The notice runs on the thread that asks for the stop, which may
    /// hold the queue's lock, so it must be short and must not call the queue.
    pub fn notify_on_stop(&self, notice: impl FnOnce() + Send + 'static) {
        let mut notices = self.stop.notices();
        if self.stop.asked.load(Ordering::SeqCst) {
            drop(notices);
            notice();
        } else {
            notices.push(Box::new(notice));
        }
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
    /// Enqueued, waiting for the jobs it conflicts with.
    Queued,
    /// A worker thread runs it.
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
