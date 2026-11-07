# CORE-007 – Job queue

- Status: implemented
- Module: core (`jobs` service)
- Origin: J1, J3
- Depends on: –

## Purpose

Long work such as an import must not keep a frontend waiting. The job queue runs jobs
in the background, in parallel where they do not get in each other's way, and reports
how each one stands, so the frontend stays free to react to the person and the devices
(ADR 0035, ADR 0036).

## Behaviour

A **job** is background work in the queue. Whoever enqueues it gets back its **job
id** at once. Job ids are numbered 1, 2, 3, … in the order the jobs were enqueued, per
queue.

**Resources.** A job names the **resources** it uses when it is enqueued, as opaque
resource keys; the queue only compares them. Two jobs **conflict** when they name a
common resource. A job that names no resource conflicts with no job.

**Scheduling.** Each running job runs on a worker thread of its own. A queued job starts
as soon as it conflicts with no running job and with no job enqueued before it that
still waits. So jobs that conflict run one after another, in the order they were
enqueued, and a later job overtakes an earlier waiting one only when it conflicts with
neither that job nor any running job. There is no limit on the number of jobs that run
at the same time beyond this rule. Enqueueing, stopping and removing return without
waiting for a job.

**Job states.** The queue reports each change of a job's state, with its job id, over a
channel. The states of one job are reported in this order:

| State | Meaning |
|---|---|
| `Queued` | Enqueued, waiting for the jobs it conflicts with |
| `Running` | A worker thread runs it |
| `Finished` | It ran to its end |
| `Stopped` | It was asked to stop while it ran, and ended early |
| `Failed` | It ended with an error, or stopped unexpectedly (a panic); the reason is a text |
| `Removed` | It was removed before it ran, and never ran |

Every job ends in exactly one of `Finished`, `Stopped`, `Failed` and `Removed`, and no
state is reported for it after that.

**Stopping and removing.** A running job is asked to stop; it decides where it can stop
safely and ends as `Stopped` there, or as `Finished` when it was already at its end. A
job that waits for something, such as an answer, can ask to be told when it is asked
to stop, so that it does not wait on. A queued job is removed and never runs. Asking to stop or remove a job that has already
ended, or a job id the queue does not know, changes nothing and says so.

**Failures.** A job that fails, or panics, ends as `Failed`; the queue goes on with the
next job.

**Shutting down.** When the queue is shut down, every running job is asked to stop, the
queued jobs are removed, and the shutdown returns once every running job has ended.

What a job reports beyond its state, such as the progress and result of an import, it
reports itself to whoever enqueued it (IMPORT-015).

## Acceptance criteria

### CORE-007.1 – Job ids count from 1
- **Given** a new queue
- **When** three jobs are enqueued
- **Then** their job ids are 1, 2 and 3

### CORE-007.2 – A job is reported queued, running and finished
- **Given** a queue
- **When** a job is enqueued and runs to its end
- **Then** the states `Queued`, `Running` and `Finished` are reported for its job id, in
  this order, and nothing after

### CORE-007.3 – Conflicting jobs run one after another in order
- **Given** a running job 1, and jobs 2 and 3 enqueued after it, all three naming the
  same resource
- **When** the jobs run
- **Then** job 2 is reported `Running` only after job 1 ended, and job 3 only after job
  2 ended

### CORE-007.4 – Enqueueing does not wait
- **Given** a running job that waits until the test lets it go on
- **When** a second job is enqueued
- **Then** the enqueue returns its job id and the second job is reported `Queued` while
  the first is still running

### CORE-007.5 – A running job is stopped where it can stop
- **Given** a running job that asks whether to go on before each of its steps
- **When** it is asked to stop
- **Then** it ends at its next step as `Stopped`, and the next job runs

### CORE-007.6 – A queued job is removed
- **Given** a running job and a queued job 2
- **When** job 2 is removed
- **Then** job 2 is reported `Removed`, never `Running`, and does not run

### CORE-007.7 – Stopping or removing an ended or unknown job changes nothing
- **Given** a job that has ended, and a job id the queue does not know
- **When** either is asked to stop or to be removed
- **Then** the answer says the job is not queued or running, and no state is reported

### CORE-007.8 – A failing job does not stop the queue
- **Given** a job that ends with an error, and a job 2 after it
- **When** the jobs run
- **Then** the first is reported `Failed` with the error's text, and job 2 runs to
  `Finished`

### CORE-007.9 – A panicking job does not stop the queue
- **Given** a job that panics, and a job 2 after it
- **When** the jobs run
- **Then** the first is reported `Failed` with a text saying it stopped unexpectedly,
  and job 2 runs to `Finished`

### CORE-007.10 – Shutting down
- **Given** a running job that asks whether to go on before each of its steps, and a
  queued job 2
- **When** the queue is shut down
- **Then** job 2 is reported `Removed`, the running job ends as `Stopped`, and the
  shutdown returns after both are reported

### CORE-007.11 – A job asked to stop at its end finishes
- **Given** a running job that has done its last step and does not ask again whether
  to go on
- **When** it is asked to stop
- **Then** it ends as `Finished`

### CORE-007.12 – Jobs without a common resource run in parallel
- **Given** a running job 1 that names resource `a` and waits until the test lets it
  go on, and a job 2 that names resource `b`
- **When** job 2 is enqueued
- **Then** job 2 is reported `Running` and runs to `Finished` while job 1 still runs

### CORE-007.13 – A waiting job overtakes only what it does not conflict with
- **Given** a running job 1 that names `a`, and jobs enqueued after it: job 2 naming
  `a` and `b`, job 3 naming `b`, and job 4 naming `c`
- **When** the jobs run
- **Then** job 4 is reported `Running` while job 1 runs, job 2 only after job 1 ended,
  and job 3 only after job 2 ended

### CORE-007.14 – A job without resources starts at once
- **Given** a running job 1 that names `a`, and a queued job 2 that names `a`
- **When** a job 3 that names no resource is enqueued
- **Then** job 3 is reported `Running` while job 1 runs and job 2 waits

### CORE-007.15 – Shutting down stops every running job
- **Given** two running jobs that ask whether to go on before each of their steps
- **When** the queue is shut down
- **Then** both end as `Stopped`, and the shutdown returns after both are reported

### CORE-007.16 – A waiting job is told that it is asked to stop
- **Given** a running job that waits until it is told that it is asked to stop
- **When** it is asked to stop
- **Then** it is told, and ends as `Stopped`; a job asked to stop before it asks to be
  told is told at once

## Out of scope

- What the resources of an import job are, and reserving sequence numbers and names
  at enqueue time: IMPORT-015, IMPORT-002.
- Pausing, reordering and showing jobs in the window header: J4.
- A queue that survives a restart, and a `JobStore` port: J6.
- Priorities and pausing low-priority jobs: J8.
- Kernel events (`JobFinished`) and commands (`EnqueueImport`): with the module
  contract (ADR 0035 §8).

## History

| Date | Change |
|---|---|
| 2026-11-07 | Created (J1, ADR 0035) |
| 2026-11-07 | Jobs that name no common resource run in parallel, each on a worker thread of its own; a job can be told when it is asked to stop (J3, ADR 0036); criteria 12–16 |
| 2026-11-07 | Implemented (J3) |
