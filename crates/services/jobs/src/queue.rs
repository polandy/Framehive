// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The job queue: enqueue, stop, remove and shut down, none of which waits for a job
//! (CORE-007, ADR 0035 §2).

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

use crate::job::{Job, JobControl, JobId, JobState, JobUpdate};
use crate::worker::run_worker;

/// The job asked for is neither queued nor running: it has ended, or the queue does
/// not know it. Nothing changed (CORE-007.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{0} is not queued or running")]
pub struct NotQueuedOrRunning(pub JobId);

/// Runs jobs one after another, in the order they were enqueued, on a worker thread
/// of its own (glossary: Job queue).
///
/// Each change of a job's state is sent, as a [`JobUpdate`], to the receiver that
/// [`JobQueue::start`] returns. Dropping the queue shuts it down.
pub struct JobQueue {
    shared: Arc<Shared>,
    last_id: u64,
    worker: Option<JoinHandle<()>>,
}

/// What the queue and its worker share.
pub(crate) struct Shared {
    lineup: Mutex<Lineup>,
    /// Wakes the worker when a job is enqueued or the queue shuts down.
    pub(crate) wake: Condvar,
}

/// The jobs that wait and the one that runs. Every state is sent while the lineup is
/// locked, so the states of each job arrive in the order they happened.
pub(crate) struct Lineup {
    pub(crate) waiting: VecDeque<(JobId, Box<dyn Job>)>,
    pub(crate) running: Option<(JobId, JobControl)>,
    /// The queue is shutting down: the worker ends once the lineup is empty.
    pub(crate) closing: bool,
    updates: Sender<JobUpdate>,
}

impl Lineup {
    /// Reports `state` for `job`. Nobody listening is not an error: the jobs still run.
    pub(crate) fn report(&self, job: JobId, state: JobState) {
        let _ = self.updates.send(JobUpdate { job, state });
    }

    /// Removes the waiting job `id`, if it waits, and reports it `Removed`.
    fn remove_waiting(&mut self, id: JobId) -> bool {
        let Some(position) = self.waiting.iter().position(|(job, _)| *job == id) else {
            return false;
        };
        self.waiting.remove(position);
        self.report(id, JobState::Removed);
        true
    }

    /// Asks the running job to stop, if it is `id`.
    fn stop_running(&self, id: JobId) -> bool {
        match &self.running {
            Some((running, control)) if *running == id => {
                control.request_stop();
                true
            }
            _ => false,
        }
    }
}

impl Shared {
    /// The lineup, also after a job panicked while the worker held it: the lineup
    /// itself is changed only by the short steps of this crate, which do not panic.
    pub(crate) fn lineup(&self) -> MutexGuard<'_, Lineup> {
        self.lineup.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl JobQueue {
    /// Starts an empty queue and its worker thread, and returns the queue with the
    /// receiver of its job updates.
    #[must_use]
    pub fn start() -> (JobQueue, Receiver<JobUpdate>) {
        let (updates, receiver) = mpsc::channel();
        let shared = Arc::new(Shared {
            lineup: Mutex::new(Lineup {
                waiting: VecDeque::new(),
                running: None,
                closing: false,
                updates,
            }),
            wake: Condvar::new(),
        });
        let worker_shared = Arc::clone(&shared);
        let worker = std::thread::spawn(move || run_worker(&worker_shared));
        let queue = JobQueue {
            shared,
            last_id: 0,
            worker: Some(worker),
        };
        (queue, receiver)
    }

    /// Puts `job` at the end of the queue, reports it `Queued` and returns its job id
    /// at once.
    pub fn enqueue(&mut self, job: Box<dyn Job>) -> JobId {
        self.last_id += 1;
        let id = JobId::new(self.last_id);
        let mut lineup = self.shared.lineup();
        lineup.report(id, JobState::Queued);
        lineup.waiting.push_back((id, job));
        self.shared.wake.notify_all();
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

    /// Removes the queued jobs, asks the running job to stop, and returns once it has
    /// ended and its state is reported.
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
        let Some(worker) = self.worker.take() else {
            return;
        };
        {
            let mut lineup = self.shared.lineup();
            lineup.closing = true;
            // The running job is asked first: a frontend that sees a queued job
            // reported removed can rely on the running one being asked to stop.
            if let Some((_, control)) = &lineup.running {
                control.request_stop();
            }
            while let Some((id, _job)) = lineup.waiting.pop_front() {
                lineup.report(id, JobState::Removed);
            }
            self.shared.wake.notify_all();
        }
        // The worker catches the panics of the jobs, so it does not end by one.
        let _ = worker.join();
    }
}

impl Drop for JobQueue {
    fn drop(&mut self) {
        self.close();
    }
}
