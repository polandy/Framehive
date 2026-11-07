// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The job queue: enqueue, stop, remove and shut down, none of which waits for a job
//! (CORE-007, ADR 0035, ADR 0036).

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

use crate::job::{Job, JobControl, JobId, JobState, JobUpdate, ResourceKey};
use crate::worker::start_ready_jobs;

/// The job asked for is neither queued nor running: it has ended, or the queue does
/// not know it. Nothing changed (CORE-007.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{0} is not queued or running")]
pub struct NotQueuedOrRunning(pub JobId);

/// Runs jobs in the background, each on a worker thread of its own; jobs that name a
/// common resource run one after another, in the order they were enqueued (glossary:
/// Job queue).
///
/// Each change of a job's state is sent, as a [`JobUpdate`], to the receiver that
/// [`JobQueue::start`] returns. Dropping the queue shuts it down.
pub struct JobQueue {
    shared: Arc<Shared>,
    last_id: u64,
    closed: bool,
}

/// What the queue and its worker threads share.
pub(crate) struct Shared {
    lineup: Mutex<Lineup>,
}

/// A job that waits, with the resources it named when it was enqueued.
pub(crate) struct Waiting {
    pub(crate) id: JobId,
    pub(crate) job: Box<dyn Job>,
    pub(crate) resources: Vec<ResourceKey>,
}

/// A job that a worker thread runs.
pub(crate) struct Running {
    pub(crate) id: JobId,
    pub(crate) control: JobControl,
    pub(crate) resources: Vec<ResourceKey>,
}

/// The jobs that wait and the ones that run. Every state is sent while the lineup is
/// locked, so the states of each job arrive in the order they happened.
pub(crate) struct Lineup {
    /// In the order the jobs were enqueued.
    pub(crate) waiting: VecDeque<Waiting>,
    pub(crate) running: Vec<Running>,
    /// The worker threads that may still run; finished ones are dropped now and then.
    pub(crate) workers: Vec<JoinHandle<()>>,
    updates: Sender<JobUpdate>,
}

impl Lineup {
    /// Reports `state` for `job`. Nobody listening is not an error: the jobs still run.
    pub(crate) fn report(&self, job: JobId, state: JobState) {
        let _ = self.updates.send(JobUpdate { job, state });
    }

    /// Removes the waiting job `id`, if it waits, and reports it `Removed`.
    fn remove_waiting(&mut self, id: JobId) -> bool {
        let Some(position) = self.waiting.iter().position(|waiting| waiting.id == id) else {
            return false;
        };
        self.waiting.remove(position);
        self.report(id, JobState::Removed);
        true
    }

    /// Asks the running job `id` to stop, if it runs.
    fn stop_running(&self, id: JobId) -> bool {
        match self.running.iter().find(|running| running.id == id) {
            Some(running) => {
                running.control.request_stop();
                true
            }
            None => false,
        }
    }
}

impl Shared {
    /// The lineup, also after a job panicked while a worker held it: the lineup itself
    /// is changed only by the short steps of this crate, which do not panic.
    pub(crate) fn lineup(&self) -> MutexGuard<'_, Lineup> {
        self.lineup.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl JobQueue {
    /// Starts an empty queue, and returns it with the receiver of its job updates.
    #[must_use]
    pub fn start() -> (JobQueue, Receiver<JobUpdate>) {
        let (updates, receiver) = mpsc::channel();
        let shared = Arc::new(Shared {
            lineup: Mutex::new(Lineup {
                waiting: VecDeque::new(),
                running: Vec::new(),
                workers: Vec::new(),
                updates,
            }),
        });
        let queue = JobQueue {
            shared,
            last_id: 0,
            closed: false,
        };
        (queue, receiver)
    }

    /// Puts `job` at the end of the queue, reports it `Queued`, starts it if it
    /// conflicts with no job before it (CORE-007, "Scheduling"), and returns its job id
    /// at once.
    pub fn enqueue(&mut self, job: Box<dyn Job>) -> JobId {
        self.last_id += 1;
        let id = JobId::new(self.last_id);
        let resources = job.resources();
        let mut lineup = self.shared.lineup();
        lineup.report(id, JobState::Queued);
        lineup.waiting.push_back(Waiting { id, job, resources });
        start_ready_jobs(&self.shared, &mut lineup);
        id
    }

    /// Stops the job `id`: a running job is asked to stop where it can, a queued job
    /// is removed and never runs. It does not wait for the job.
    ///
    /// # Errors
    ///
    /// Returns [`NotQueuedOrRunning`] if the job has ended or is unknown.
    pub fn stop(&self, id: JobId) -> Result<(), NotQueuedOrRunning> {
        self.withdraw(id)
    }

    /// Removes the queued job `id`, so it never runs. A job that started meanwhile is
    /// asked to stop instead, so a frontend that showed it queued gets what it asked
    /// for. It does not wait for the job.
    ///
    /// # Errors
    ///
    /// Returns [`NotQueuedOrRunning`] if the job has ended or is unknown.
    pub fn remove(&self, id: JobId) -> Result<(), NotQueuedOrRunning> {
        self.withdraw(id)
    }

    /// Asks every running job to stop, removes the queued jobs, and returns once every
    /// running job has ended and its state is reported.
    pub fn shut_down(mut self) {
        self.close();
    }

    fn withdraw(&self, id: JobId) -> Result<(), NotQueuedOrRunning> {
        let mut lineup = self.shared.lineup();
        if lineup.remove_waiting(id) || lineup.stop_running(id) {
            Ok(())
        } else {
            Err(NotQueuedOrRunning(id))
        }
    }

    fn close(&mut self) {
        if std::mem::replace(&mut self.closed, true) {
            return;
        }
        let workers = {
            let mut lineup = self.shared.lineup();
            // The running jobs are asked first: a frontend that sees a queued job
            // reported removed can rely on the running ones being asked to stop.
            for running in &lineup.running {
                running.control.request_stop();
            }
            while let Some(waiting) = lineup.waiting.pop_front() {
                lineup.report(waiting.id, JobState::Removed);
            }
            // With nothing waiting, no worker starts another one, so these are all.
            std::mem::take(&mut lineup.workers)
        };
        for worker in workers {
            // A worker catches the panic of its job, so it does not end by one.
            let _ = worker.join();
        }
    }
}

impl Drop for JobQueue {
    fn drop(&mut self) {
        self.close();
    }
}
