// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The job queue (CORE-007). The jobs of these tests wait on channels the test opens,
//! so every order is fixed without sleeping. Unless a test says otherwise, its jobs
//! name one common resource, `disk`, so they run one after another.

#![allow(clippy::unwrap_used, clippy::panic)]

mod support;

use jobs::{JobEnd, JobId, JobQueue, JobState, JobUpdate, NotQueuedOrRunning};
use support::*;

/// spec: CORE-007.1
#[test]
fn job_ids_count_from_one() {
    let (mut queue, _updates) = JobQueue::start();

    let ids: Vec<u64> = (0..3).map(|_| queue.enqueue(finishes()).number()).collect();

    assert_eq!(ids, [1, 2, 3]);
}

/// spec: CORE-007.2
#[test]
fn a_job_is_reported_queued_running_and_finished() {
    let (mut queue, updates) = JobQueue::start();
    queue.enqueue(finishes());
    assert_eq!(next(&updates), update(1, JobState::Queued));
    assert_eq!(next(&updates), update(1, JobState::Running));
    assert_eq!(next(&updates), update(1, JobState::Finished));

    assert_eq!(all_updates(queue, &updates), []);
}

/// spec: CORE-007.3
#[test]
fn a_job_runs_only_after_the_one_before_it_ended() {
    let (mut queue, updates) = JobQueue::start();
    let (first, handles) = steps(1);
    queue.enqueue(first);
    handles.at_step.recv().unwrap();
    queue.enqueue(finishes());
    queue.enqueue(finishes());
    handles.gate.send(()).unwrap();
    let mut seen = Vec::new();
    while seen.last() != Some(&update(3, JobState::Finished)) {
        seen.push(next(&updates));
    }

    let running_and_ended: Vec<JobUpdate> = seen
        .into_iter()
        .filter(|update| update.state != JobState::Queued)
        .collect();

    assert_eq!(
        running_and_ended,
        [
            update(1, JobState::Running),
            update(1, JobState::Finished),
            update(2, JobState::Running),
            update(2, JobState::Finished),
            update(3, JobState::Running),
            update(3, JobState::Finished),
        ]
    );
}

/// spec: CORE-007.4
#[test]
fn enqueueing_does_not_wait_for_the_running_job() {
    let (mut queue, updates) = JobQueue::start();
    let (first, handles) = steps(1);
    queue.enqueue(first);
    handles.at_step.recv().unwrap();

    let second = queue.enqueue(finishes());

    assert_eq!(second, JobId::new(2));
    assert_eq!(next(&updates), update(1, JobState::Queued));
    assert_eq!(next(&updates), update(1, JobState::Running));
    assert_eq!(next(&updates), update(2, JobState::Queued));
    handles.gate.send(()).unwrap();
    assert_eq!(next(&updates), update(1, JobState::Finished));
}

/// spec: CORE-007.5
#[test]
fn a_running_job_is_stopped_where_it_can_stop() {
    let (mut queue, updates) = JobQueue::start();
    let (first, handles) = steps(3);
    let first = queue.enqueue(first);
    queue.enqueue(finishes());
    assert_eq!(handles.at_step.recv(), Ok(0));

    queue.stop(first).unwrap();
    handles.gate.send(()).unwrap();
    while next(&updates) != update(2, JobState::Finished) {}

    assert!(handles.at_step.try_recv().is_err(), "no second step");
    let reported = all_updates(queue, &updates);
    assert_eq!(reported, []);
}

/// spec: CORE-007.5
#[test]
fn a_stopped_job_is_reported_stopped_before_the_next_job_runs() {
    let (mut queue, updates) = JobQueue::start();
    let (first, handles) = steps(3);
    let first = queue.enqueue(first);
    handles.at_step.recv().unwrap();
    queue.enqueue(finishes());

    queue.stop(first).unwrap();
    handles.gate.send(()).unwrap();

    let reported = [(); 6].map(|()| next(&updates));
    assert_eq!(
        reported,
        [
            update(1, JobState::Queued),
            update(1, JobState::Running),
            update(2, JobState::Queued),
            update(1, JobState::Stopped),
            update(2, JobState::Running),
            update(2, JobState::Finished),
        ]
    );
}

/// spec: CORE-007.6
#[test]
fn a_queued_job_is_removed_and_never_runs() {
    let (mut queue, updates) = JobQueue::start();
    let (first, handles) = steps(1);
    queue.enqueue(first);
    handles.at_step.recv().unwrap();
    let (second, second_handles) = steps(1);
    let second = queue.enqueue(second);

    queue.remove(second).unwrap();
    handles.gate.send(()).unwrap();
    let reported = all_updates(queue, &updates);

    assert_eq!(
        reported,
        [
            update(1, JobState::Queued),
            update(1, JobState::Running),
            update(2, JobState::Queued),
            update(2, JobState::Removed),
            update(1, JobState::Finished),
        ]
    );
    assert!(
        second_handles.at_step.try_recv().is_err(),
        "job 2 never ran"
    );
}

/// spec: CORE-007.7
#[test]
fn stopping_or_removing_an_ended_or_unknown_job_changes_nothing() {
    let (mut queue, updates) = JobQueue::start();
    let ended = queue.enqueue(finishes());
    while next(&updates) != update(1, JobState::Finished) {}
    let unknown = JobId::new(7);

    let answers = [
        queue.stop(ended),
        queue.remove(ended),
        queue.stop(unknown),
        queue.remove(unknown),
    ];

    assert_eq!(
        answers,
        [
            Err(NotQueuedOrRunning(ended)),
            Err(NotQueuedOrRunning(ended)),
            Err(NotQueuedOrRunning(unknown)),
            Err(NotQueuedOrRunning(unknown)),
        ]
    );
    assert_eq!(
        NotQueuedOrRunning(unknown).to_string(),
        "job 7 is not queued or running"
    );
    assert_eq!(all_updates(queue, &updates), []);
}

/// spec: CORE-007.8
#[test]
fn a_failing_job_does_not_stop_the_queue() {
    let (mut queue, updates) = JobQueue::start();
    queue.enqueue(on(
        &["disk"],
        Ends(JobEnd::Failed("the disk is full".to_owned())),
    ));
    queue.enqueue(finishes());

    let reported = [(); 6].map(|()| next(&updates));

    assert!(
        reported.contains(&update(1, JobState::Failed("the disk is full".to_owned()))),
        "{reported:?}"
    );
    assert_eq!(reported[5], update(2, JobState::Finished));
}

/// spec: CORE-007.9
#[test]
fn a_panicking_job_does_not_stop_the_queue() {
    let (mut queue, updates) = JobQueue::start();
    queue.enqueue(on(&["disk"], Panics));
    queue.enqueue(finishes());

    let reported = [(); 6].map(|()| next(&updates));

    assert!(
        reported.contains(&update(
            1,
            JobState::Failed("the job stopped unexpectedly: broken on purpose".to_owned())
        )),
        "{reported:?}"
    );
    assert_eq!(reported[5], update(2, JobState::Finished));
}

/// spec: CORE-007.10
#[test]
fn shutting_down_removes_the_queued_jobs_and_stops_the_running_one() {
    let (mut queue, updates) = JobQueue::start();
    let (first, handles) = steps(3);
    queue.enqueue(first);
    queue.enqueue(finishes());
    handles.at_step.recv().unwrap();

    // The shutdown waits for the running job, so it runs on a thread of its own; the
    // gate opens only once the shutdown removed job 2 and asked job 1 to stop.
    let shutdown = std::thread::spawn(move || queue.shut_down());
    while next(&updates) != update(2, JobState::Removed) {}
    handles.gate.send(()).unwrap();
    shutdown.join().unwrap();

    let after_removed: Vec<JobUpdate> = updates.try_iter().collect();
    assert_eq!(after_removed, [update(1, JobState::Stopped)]);
    assert!(handles.at_step.try_recv().is_err(), "no second step");
}

/// spec: CORE-007.11
#[test]
fn a_job_asked_to_stop_after_its_last_step_finishes() {
    let (mut queue, updates) = JobQueue::start();
    let (job, handles) = steps(1);
    let job = queue.enqueue(job);
    handles.at_step.recv().unwrap();

    queue.stop(job).unwrap();
    handles.gate.send(()).unwrap();

    assert_eq!(
        all_updates(queue, &updates),
        [
            update(1, JobState::Queued),
            update(1, JobState::Running),
            update(1, JobState::Finished),
        ]
    );
}
