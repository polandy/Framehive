<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# jobs

Layer: `service` (architecture §2, §3)

## Purpose

The job queue (CORE-007, ADR 0035) runs background work one job after another on a
worker thread of its own, and reports each change of a job's state over a channel.

Boundaries:

- **Knows no work.** A `Job` owns what it needs and implements `run`; the import
  module implements it for its import, and apps create the queue and enqueue jobs.
- **Never waits for a job.** `enqueue`, `stop` and `remove` answer at once.
  Only `shut_down` (and dropping the queue) waits for the running job to end.
- **Stopping is cooperative.** A running job asks its `JobControl` at points it
  chooses; a queued job is removed and never runs.
- **A job cannot take the queue down.** An error or a panic ends that job as `Failed`
  and the next one runs.

## Public API

| Item | Meaning |
|---|---|
| `JobQueue` | Starts the worker; `enqueue`, `stop`, `remove`, `shut_down` |
| `Job`, `JobControl`, `JobEnd` | The work, how it learns to stop, how it ended |
| `JobId`, `JobState`, `JobUpdate` | The id of a job, where it stands, a reported change |
| `NotQueuedOrRunning` | The job asked for has ended or is unknown |

## Tests

`tests/job_queue.rs` fixes every order with channels the test opens; no sleeping.
