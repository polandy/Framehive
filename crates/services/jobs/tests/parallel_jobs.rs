// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Jobs of the job queue that run in parallel (CORE-007.12-16, ADR 0036). The jobs wait
//! on channels the test opens, so every order is fixed without sleeping.

#![allow(clippy::unwrap_used, clippy::panic)]

mod support;

use std::sync::mpsc::{self, Receiver, Sender};

use jobs::{Job, JobControl, JobEnd, JobQueue, JobState, JobUpdate};
use support::*;

/// The updates until `last`, `last` included.
fn updates_until(updates: &Receiver<JobUpdate>, last: &JobUpdate) -> Vec<JobUpdate> {
    let mut seen = Vec::new();
    while seen.last() != Some(last) {
        seen.push(next(updates));
    }
    seen
}

/// Where `wanted` is in `reported`.
fn position(reported: &[JobUpdate], wanted: &JobUpdate) -> usize {
    reported
        .iter()
        .position(|update| update == wanted)
        .unwrap_or_else(|| panic!("{wanted:?} not in {reported:?}"))
}

/// spec: CORE-007.12
#[test]
fn jobs_without_a_common_resource_run_in_parallel() {
    let (mut queue, updates) = JobQueue::start();
    let (first, handles) = steps_on(&["a"], 1);
    queue.enqueue(first);
    handles.at_step.recv().unwrap();

    queue.enqueue(on(&["b"], Ends(JobEnd::Finished)));
    let seen = updates_until(&updates, &update(2, JobState::Finished));

    assert!(seen.contains(&update(2, JobState::Running)), "{seen:?}");
    assert!(!seen.contains(&update(1, JobState::Finished)), "{seen:?}");
    handles.gate.send(()).unwrap();
    assert_eq!(next(&updates), update(1, JobState::Finished));
}

/// spec: CORE-007.13
#[test]
fn a_waiting_job_overtakes_only_what_it_does_not_conflict_with() {
    let (mut queue, updates) = JobQueue::start();
    let (first, first_handles) = steps_on(&["a"], 1);
    queue.enqueue(first);
    first_handles.at_step.recv().unwrap();
    let (second, second_handles) = steps_on(&["a", "b"], 1);
    let (third, third_handles) = steps_on(&["b"], 1);
    let (fourth, fourth_handles) = steps_on(&["c"], 1);
    queue.enqueue(second);
    queue.enqueue(third);
    queue.enqueue(fourth);

    fourth_handles.at_step.recv().unwrap();
    assert!(second_handles.at_step.try_recv().is_err(), "job 2 waits");
    assert!(third_handles.at_step.try_recv().is_err(), "job 3 waits");
    fourth_handles.gate.send(()).unwrap();
    first_handles.gate.send(()).unwrap();
    second_handles.at_step.recv().unwrap();
    assert!(third_handles.at_step.try_recv().is_err(), "job 3 waits");
    second_handles.gate.send(()).unwrap();
    third_handles.at_step.recv().unwrap();
    third_handles.gate.send(()).unwrap();
    let reported = updates_until(&updates, &update(3, JobState::Finished));

    let at = |number, state| position(&reported, &update(number, state));
    assert!(at(4, JobState::Running) < at(1, JobState::Finished));
    assert!(at(1, JobState::Finished) < at(2, JobState::Running));
    assert!(at(2, JobState::Finished) < at(3, JobState::Running));
}

/// spec: CORE-007.14
#[test]
fn a_job_without_resources_starts_at_once() {
    let (mut queue, updates) = JobQueue::start();
    let (first, first_handles) = steps_on(&["a"], 1);
    queue.enqueue(first);
    first_handles.at_step.recv().unwrap();
    let (second, second_handles) = steps_on(&["a"], 1);
    queue.enqueue(second);

    let (third, third_handles) = steps_on(&[], 1);
    queue.enqueue(third);

    third_handles.at_step.recv().unwrap();
    assert!(second_handles.at_step.try_recv().is_err(), "job 2 waits");
    third_handles.gate.send(()).unwrap();
    first_handles.gate.send(()).unwrap();
    second_handles.at_step.recv().unwrap();
    second_handles.gate.send(()).unwrap();
    let reported = updates_until(&updates, &update(2, JobState::Finished));
    let at = |number, state| position(&reported, &update(number, state));
    assert!(at(3, JobState::Running) < at(1, JobState::Finished));
    assert!(at(1, JobState::Finished) < at(2, JobState::Running));
}

/// spec: CORE-007.15
#[test]
fn shutting_down_stops_every_running_job() {
    let (mut queue, updates) = JobQueue::start();
    let (first, first_handles) = steps_on(&["a"], 3);
    let (second, second_handles) = steps_on(&["b"], 3);
    queue.enqueue(first);
    queue.enqueue(second);
    first_handles.at_step.recv().unwrap();
    second_handles.at_step.recv().unwrap();
    // Job 3 waits for both; its removal shows that both were asked to stop.
    queue.enqueue(on(&["a", "b"], Ends(JobEnd::Finished)));

    let shutdown = std::thread::spawn(move || queue.shut_down());
    while next(&updates) != update(3, JobState::Removed) {}
    first_handles.gate.send(()).unwrap();
    second_handles.gate.send(()).unwrap();
    shutdown.join().unwrap();

    let mut after_removed: Vec<JobUpdate> = updates.try_iter().collect();
    after_removed.sort_by_key(|update| update.job);
    assert_eq!(
        after_removed,
        [update(1, JobState::Stopped), update(2, JobState::Stopped)]
    );
}

/// A job that tells the test it waits, then waits until it is told that it is asked
/// to stop.
struct WaitsForTheStop {
    waits: Sender<()>,
}

impl Job for WaitsForTheStop {
    fn run(self: Box<Self>, control: &JobControl) -> JobEnd {
        let (told, telling) = mpsc::channel();
        control.notify_on_stop(move || {
            let _ = told.send(());
        });
        let _ = self.waits.send(());
        match telling.recv() {
            Ok(()) => JobEnd::Stopped,
            Err(_) => JobEnd::Failed("never told".to_owned()),
        }
    }
}

/// spec: CORE-007.16
#[test]
fn a_waiting_job_is_told_that_it_is_asked_to_stop() {
    let (mut queue, updates) = JobQueue::start();
    let (waits, waiting) = mpsc::channel();
    let job = queue.enqueue(Box::new(WaitsForTheStop { waits }));
    waiting.recv().unwrap();

    queue.stop(job).unwrap();

    assert_eq!(
        updates_until(&updates, &update(1, JobState::Stopped)),
        [
            update(1, JobState::Queued),
            update(1, JobState::Running),
            update(1, JobState::Stopped),
        ]
    );
}

/// spec: CORE-007.16
#[test]
fn a_job_asked_to_stop_before_it_asks_to_be_told_is_told_at_once() {
    let control = JobControl::new();
    control.request_stop();
    let (told, telling) = mpsc::channel();

    control.notify_on_stop(move || told.send(()).unwrap());

    assert_eq!(telling.try_recv(), Ok(()));
}
