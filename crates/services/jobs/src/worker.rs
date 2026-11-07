// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The worker thread: takes the jobs one by one, runs each, and reports how it ended
//! (CORE-007, ADR 0035 §2 and §6).

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::job::{Job, JobControl, JobId, JobState};
use crate::queue::Shared;

/// Runs the jobs of `shared` until the queue shuts down and no job is left.
pub(crate) fn run_worker(shared: &Shared) {
    while let Some((id, job, control)) = next_job(shared) {
        let state = run_job(job, &control);
        let mut lineup = shared.lineup();
        lineup.running = None;
        lineup.report(id, state);
    }
}

/// Waits for the next job, marks it running and reports it `Running`; `None` once the
/// queue shuts down.
fn next_job(shared: &Shared) -> Option<(JobId, Box<dyn Job>, JobControl)> {
    let mut lineup = shared.lineup();
    loop {
        if let Some((id, job)) = lineup.waiting.pop_front() {
            let control = JobControl::new();
            lineup.running = Some((id, control.clone()));
            lineup.report(id, JobState::Running);
            return Some((id, job, control));
        }
        if lineup.closing {
            return None;
        }
        lineup = shared
            .wake
            .wait(lineup)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
    }
}

/// Runs `job` and tells how it ended; a panic fails the job, not the queue (ADR 0035
/// §6). Unwinding cannot leave the queue half-changed: the job owns all it touches.
fn run_job(job: Box<dyn Job>, control: &JobControl) -> JobState {
    match catch_unwind(AssertUnwindSafe(|| job.run(control))) {
        Ok(end) => end.into(),
        Err(payload) => JobState::Failed(unexpected_stop(payload.as_ref())),
    }
}

/// The reason of a job that panicked, with the panic's message when it has one.
fn unexpected_stop(payload: &(dyn Any + Send)) -> String {
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    match message {
        Some(message) => format!("the job stopped unexpectedly: {message}"),
        None => "the job stopped unexpectedly".to_owned(),
    }
}
