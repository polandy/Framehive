// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The worker threads: the scheduling rule that starts the jobs that can start, one
//! thread per running job, and the report of how each job ended (CORE-007, ADR 0035
//! §6, ADR 0036 §1 and §3).

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use crate::job::{Job, JobControl, JobId, JobState, ResourceKey};
use crate::queue::{Lineup, Running, Shared, Waiting};

/// Starts every waiting job that conflicts with no running job and with no job
/// enqueued before it that still waits (ADR 0036 §3).
pub(crate) fn start_ready_jobs(shared: &Arc<Shared>, lineup: &mut Lineup) {
    // The resources of the earlier jobs that keep waiting.
    let mut held_back: Vec<ResourceKey> = Vec::new();
    let mut index = 0;
    while let Some(waiting) = lineup.waiting.get(index) {
        let conflicts = waiting.resources.iter().any(|resource| {
            held_back.contains(resource)
                || lineup
                    .running
                    .iter()
                    .any(|running| running.resources.contains(resource))
        });
        if conflicts {
            held_back.extend(waiting.resources.iter().cloned());
            index += 1;
        } else if let Some(waiting) = lineup.waiting.remove(index) {
            start_job(shared, lineup, waiting);
        }
    }
}

/// Reports `waiting` `Running` and runs it on a worker thread of its own.
fn start_job(shared: &Arc<Shared>, lineup: &mut Lineup, waiting: Waiting) {
    let Waiting { id, job, resources } = waiting;
    let control = JobControl::new();
    lineup.report(id, JobState::Running);
    let worker_shared = Arc::clone(shared);
    let worker_control = control.clone();
    let spawned = std::thread::Builder::new()
        .name(format!("framehive {id}"))
        .spawn(move || run_worker(&worker_shared, id, job, &worker_control));
    match spawned {
        Ok(worker) => {
            lineup.workers.retain(|worker| !worker.is_finished());
            lineup.workers.push(worker);
            lineup.running.push(Running {
                id,
                control,
                resources,
            });
        }
        Err(error) => lineup.report(
            id,
            JobState::Failed(format!("the job could not start a thread: {error}")),
        ),
    }
}

/// Runs the job `id` on its worker thread, reports how it ended, and starts the jobs
/// that waited for it.
fn run_worker(shared: &Arc<Shared>, id: JobId, job: Box<dyn Job>, control: &JobControl) {
    let state = run_job(job, control);
    let mut lineup = shared.lineup();
    lineup.running.retain(|running| running.id != id);
    lineup.report(id, state);
    start_ready_jobs(shared, &mut lineup);
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
