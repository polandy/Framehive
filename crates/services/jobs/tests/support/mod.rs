// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The jobs shared by the job queue tests (CORE-007). They wait on channels the test
//! opens, so every order is fixed without sleeping.

#![allow(
    clippy::unwrap_used,
    clippy::struct_field_names,
    reason = "test helpers"
)]
#![allow(dead_code, reason = "each test binary uses only some of the helpers")]

use std::sync::mpsc::{self, Receiver, Sender};

use jobs::{Job, JobControl, JobEnd, JobId, JobQueue, JobState, JobUpdate, ResourceKey};

/// A job that ends at once as it is told.
pub(crate) struct Ends(pub(crate) JobEnd);

impl Job for Ends {
    fn run(self: Box<Self>, _control: &JobControl) -> JobEnd {
        self.0
    }
}

/// A job that panics.
pub(crate) struct Panics;

impl Job for Panics {
    fn run(self: Box<Self>, _control: &JobControl) -> JobEnd {
        panic!("broken on purpose");
    }
}

/// A job of `steps` steps. Before each step it asks whether to go on; then it says
/// which step it is at and waits until the test opens the gate.
pub(crate) struct Steps {
    pub(crate) steps: usize,
    pub(crate) at_step: Sender<usize>,
    pub(crate) gate: Receiver<()>,
}

impl Job for Steps {
    fn run(self: Box<Self>, control: &JobControl) -> JobEnd {
        for step in 0..self.steps {
            if !control.should_continue() {
                return JobEnd::Stopped;
            }
            let _ = self.at_step.send(step);
            let _ = self.gate.recv();
        }
        JobEnd::Finished
    }
}

/// A job that names `resources`.
pub(crate) struct Using {
    pub(crate) job: Box<dyn Job>,
    pub(crate) resources: Vec<ResourceKey>,
}

impl Job for Using {
    fn run(self: Box<Self>, control: &JobControl) -> JobEnd {
        self.job.run(control)
    }

    fn resources(&self) -> Vec<ResourceKey> {
        self.resources.clone()
    }
}

/// `job`, naming the resources `names`.
pub(crate) fn on(names: &[&str], job: impl Job) -> Box<dyn Job> {
    Box::new(Using {
        job: Box::new(job),
        resources: names.iter().copied().map(ResourceKey::new).collect(),
    })
}

/// The handles a test holds of a [`Steps`] job.
pub(crate) struct StepHandles {
    pub(crate) at_step: Receiver<usize>,
    pub(crate) gate: Sender<()>,
}

pub(crate) fn steps(count: usize) -> (Box<dyn Job>, StepHandles) {
    steps_on(&["disk"], count)
}

/// A [`Steps`] job of `count` steps that names the resources `names`.
pub(crate) fn steps_on(names: &[&str], count: usize) -> (Box<dyn Job>, StepHandles) {
    let (at_step_sender, at_step) = mpsc::channel();
    let (gate, gate_receiver) = mpsc::channel();
    let job = Steps {
        steps: count,
        at_step: at_step_sender,
        gate: gate_receiver,
    };
    (on(names, job), StepHandles { at_step, gate })
}

pub(crate) fn finishes() -> Box<dyn Job> {
    on(&["disk"], Ends(JobEnd::Finished))
}

pub(crate) fn update(number: u64, state: JobState) -> JobUpdate {
    JobUpdate {
        job: JobId::new(number),
        state,
    }
}

/// Shuts the queue down and returns every update reported.
pub(crate) fn all_updates(queue: JobQueue, updates: &Receiver<JobUpdate>) -> Vec<JobUpdate> {
    queue.shut_down();
    updates.try_iter().collect()
}

/// Waits for the next update.
pub(crate) fn next(updates: &Receiver<JobUpdate>) -> JobUpdate {
    updates.recv().unwrap()
}
