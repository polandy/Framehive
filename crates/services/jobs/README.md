<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# jobs

Layer: `service` (architecture §2, §3)

## Purpose

The job queue (CORE-007, ADR 0035, ADR 0036) runs background work, each running job on
a worker thread of its own, and reports each change of a job's state over a channel.
Jobs that name a common resource run one after another in enqueue order; jobs that
share none run in parallel.

Boundaries:

- **Knows no work.** A `Job` owns what it needs and implements `run`; the import
  module implements it for its import, and apps create the queue and enqueue jobs.
- **Resources are opaque.** `Job::resources` names `ResourceKey`s, which the queue
  only compares. A waiting job starts when it shares no key with a running job or an
  earlier waiting job; a job without keys starts at once. There is no other limit.
- **Never waits for a job.** `enqueue`, `stop` and `remove` answer at once.
  Only `shut_down` (and dropping the queue) waits: it asks every running job to stop,
  removes the waiting ones and joins every worker thread.
- **Stopping is cooperative.** A running job asks its `JobControl` at points it
  chooses, or has `notify_on_stop` wake it while it waits; a queued job is removed and
  never runs.
- **A job cannot take the queue down.** An error or a panic ends that job as `Failed`
  and the other jobs go on.

## Public API

| Item | Meaning |
|---|---|
| `JobQueue` | Takes jobs and starts them; `enqueue`, `stop`, `remove`, `shut_down` |
| `Job`, `JobControl`, `JobEnd` | The work, how it learns to stop, how it ended |
| `ResourceKey` | Something a job uses that no other job may use at the same time |
| `JobId`, `JobState`, `JobUpdate` | The id of a job, where it stands, a reported change |
| `NotQueuedOrRunning` | The job asked for has ended or is unknown |

## Tests

`tests/job_queue.rs` (one job after another) and `tests/parallel_jobs.rs` (jobs in
parallel, ADR 0036) fix every order with channels the test opens; no sleeping.
