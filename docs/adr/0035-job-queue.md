# 0035 – Job queue on a worker thread

- Status: accepted
- Date: 2026-11-07

## Context

An import must run as a job in a queue in the core, so that no frontend waits on it
(J1). The architecture already names a `jobs` service ("queue, scheduler, priorities,
resume", §2) and the extension point `jobs::JobKind` (§5), but neither exists yet. The
kernel is still a scaffold without the module contract, events or commands.

Today `ImportRun` runs on the caller's thread and borrows its ports (`&dyn FileSystem`,
`&dyn ImportHistory`, …). It reports through a `ProgressSink`, and the guided import
polls the devices from inside that sink between two files (CLI-003). The adapters the
import needs can move between threads: the SQLite connection and the gphoto2 session
are `Send`, though not `Sync`.

Later requirements build on the queue: planning the next source while an import runs
(J2), parallel jobs (J3), an activity indicator with pause and reorder (J4) and a queue
that survives a restart (J6). J1 only lays the ground for them.

## Decision

### 1. A `jobs` service crate

A new crate `crates/services/jobs` (layer `service`) holds `JobQueue`, `JobId`,
`JobState` and the `Job` trait (CORE-007). It knows nothing about imports. The import
module implements `Job` for an `ImportJob` (IMPORT-015); the apps create the queue
and enqueue jobs.

### 2. One worker thread, first in, first out

The queue runs its jobs on one worker thread of its own, one after another in the
order they were enqueued. Enqueueing, stopping and removing never wait for a job.
Running jobs in parallel is J3.

### 3. std threads and channels, no async runtime

The worker is a `std::thread`. The queue and the worker share the waiting jobs behind
a `Mutex` and a `Condvar`, so that stop and remove answer at once (CORE-007.7); state
reports travel over a `std::sync::mpsc` channel. The import is blocking file I/O, so an async runtime would
add a dependency (ADR 0009) without a gain.

### 4. A job owns what it needs

A job is `Send + 'static`: it owns its settings and its adapters, which move to the
worker with it. A job reports its own progress and result to whoever enqueued it over
a channel of its own type; the queue itself reports only the state of each job. What a
job borrowed from its enqueuer, such as an opened camera, it hands back with its
result, so the enqueuer can release it.

### 5. Stopping is cooperative

A running job is never interrupted. It asks its `JobControl` whether to go on at
points it chooses; the import asks before each file, as IMPORT-004 "Stopping" already
does. A job that has not started is removed from the queue and never runs.

### 6. A panic fails the job, not the queue

The worker runs each job under `std::panic::catch_unwind`. A job that panics ends as
failed, and the queue goes on with the next one.

### 7. Numbered per queue, not persisted

`JobId`s are numbered 1, 2, 3, … per queue, which needs no ID port and keeps tests
deterministic. There is no `JobStore` port yet: a queue lives as long as the
application, and an interrupted import is resumed by running it again (IMPORT-009).
J6 adds the port and a persistent numbering.

### 8. No kernel events or commands yet

The queue reports over its own channel. `JobFinished` and `EnqueueImport`
(architecture §4.3) follow with the module contract, and need their own ADR then.

## Alternatives

- **An async runtime (`tokio`):** a large dependency for work that is blocking file I/O
  anyway.
- **A thread per job instead of a queue:** simplest, but gives no order, no place to
  pause or reorder later (J4) and no limit on parallel imports to one disk.
- **Jobs that borrow their ports (`std::thread::scope`):** keeps `ImportRun` as it is,
  but ties the queue to one scope of the caller, so a GUI could not keep it across
  views.
- **Kill a running job:** threads cannot be killed safely in Rust, and a copy cut off
  halfway would leave a `.part` file and a half-recorded history.

## Consequences

- `ImportRun` keeps its borrowing API; `ImportJob` owns the adapters and builds an
  `ImportRun` on the worker thread.
- The import job of the guided import asks the dialog before each file whether to go
  on and waits for the answer (IMPORT-015.9); the dialog polls the devices on its own
  thread before it answers. No timeout is involved, so the golden tests stay
  deterministic, and a pulled card or camera stops the job as before (CLI-003,
  CLI-014).
- Adapters that are not `Send` cannot be used by a job. All current ones are; an opened
  camera is now `Send` (`Cameras::open`), and the command line shares it with the
  dialog behind a lock.
- Until J3, a second import waits for the first, even on different disks.
