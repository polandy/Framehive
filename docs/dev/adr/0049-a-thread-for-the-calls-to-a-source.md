# 0049 – A thread for the calls to a source

- Status: accepted
- Date: 2026-11-19

## Context

IMPORT-022 gives every call the import makes to a source a stall limit: a call that does
not return within it fails its file, the source is stopped for the rest of the run, and
the other sources go on. Its out-of-scope section leaves open how the import stops
waiting for a call and how tests measure the limit without the wall clock (ADR 0048 §4).

The import reads a source only through the `ReadOnlyFileSystem` port of the `files`
service, for a card as for a camera or phone (ADR 0034). Its calls are synchronous: a
read from a USB device that stops answering blocks in the kernel, and std has no way to
cancel it or to give it a time limit. A thread that is blocked in such a call cannot be
killed safely (ADR 0035), and the architecture cancels work with tokens, never by
killing threads (architecture §6). The only way to stop waiting is to make the call on
another thread and to leave that thread behind.

Tests are deterministic (CLAUDE.md rule 4, ADR 0006): they never sleep and never read
the wall clock, so a test of a 30-second limit must not take 30 seconds.

## Decision

### 1. A source thread behind a `ReadOnlyFileSystem` decorator

The `files` service gets `StallGuard`, a `ReadOnlyFileSystem` that owns the source
(`Box<dyn ReadOnlyFileSystem + Send>`) and starts one thread of its own for it, the
*source thread*. Each call of the port is sent to the source thread as a request, the
source thread makes the calls one after another and sends each answer back, and the
caller waits for the answer.

A file opened with `open_read` stays on the source thread. `StallGuard` returns a
`ReadSeek` that sends each `read` and `seek` as a request; a read answers with the
bytes it got, so each buffer is copied once more, which costs little next to a read
from a device. Because all calls of a source run on one thread, they never overlap,
which a camera needs anyway (ADR 0036).

The import wraps each source of a run in a `StallGuard` of its own, with the limit of
the profile, so a hanging source stops only itself (IMPORT-022.4). Destinations are not
wrapped: their calls have no limit.

### 2. A call that exceeds the limit stops the source

The deadline of a call is fixed when its request is sent. When it passes before the
answer, the call returns an `io::Error` of kind `TimedOut` whose message is
`the source did not answer for <n> seconds`, and the guard marks the source stopped.
From then on every call returns at once, without a request, an `io::Error` whose
message is `the source stopped answering`. The import already reports a failed source
call as `cannot read the source: <path>: <error>` and a failed scan as
`<path>: <error>`, so it needs no new error of its own; only the run must stop calling
a source once it reports the stopped error, which the guard ensures by itself.

The source thread of a stopped source stays blocked in its call. Nothing joins it:
dropping the guard closes its request channel, and a thread that is not blocked ends,
dropping the source and so releasing a camera. A blocked thread ends with the process.
A stopped source sends no more requests, so it holds at most one blocked call
(IMPORT-022).

### 3. A `Timer` port for the waiting

Waiting with a limit needs monotonic time, which the services must not read themselves
(architecture §7). The `files` service gets a port `Timer`, `Send + Sync`, with two
calls: one that returns the deadline `limit` from now, and one that waits until an
answer arrives or the deadline passes and says which came first. The answer is signalled
through a small type of the service (a flag behind a mutex and condition variable), so
the port stays object-safe and knows nothing of requests.

- **`adapter-timer`**, made with `cargo xtask new-adapter timer std`, implements it with
  `std::time::Instant` and `Condvar::wait_timeout`. It needs no dependency. It passes
  the port's conformance suite, which checks that an answer before the deadline wins
  and that a deadline with no answer ends the wait.
- **`files::testing::ManualTimer`** implements it with time that passes only when the
  test advances it, like `ManualClock`. Advancing wakes every wait whose deadline has
  passed.

The apps build the real timer and hand it to the import with the other ports.

### 4. Tests advance the time, never wait for it

`MemoryFileSystem` gets two faults:

- a call that hangs: the chosen call (a read of a file, or the listing of a folder)
  signals that it has begun and then blocks until the test releases it at its end;
- a slow source: each read advances a `ManualTimer` by a given duration before it
  answers.

A test of a hang waits for the signal that the call has begun, then advances the
`ManualTimer` by the limit; because the deadline was fixed before the request was sent,
the call fails at exactly the limit, never sooner (IMPORT-022.1, .5, .6). A test of a
slow source advances the timer by less than the limit per read and expects the copy to
succeed (IMPORT-022.3). No test sleeps or reads the wall clock.

The binary test of T5 (CLI-025) uses the real timer with the profile key set to 1
second, the shortest the spec allows: its outcome does not depend on how long the
process takes, only on the hang, which never returns.

## Alternatives considered

- **A thread per call.** Spawning a thread for each read costs more than many reads
  and lets the calls of one source overlap, which a camera does not allow. Rejected.
- **A time limit in each adapter.** std offers none for a read from a file, a
  non-blocking read does not apply to regular files, and libgphoto2's own timeouts do
  not cover a USB mass-storage card. Each adapter would need its own native code.
  Rejected.
- **An async runtime.** It would make no blocking read cancellable either (its file
  reads run on blocking threads), and the architecture allows one only where an adapter
  strictly needs it (ADR 0009). Rejected.
- **A watchdog thread per run.** It could notice a hang, but the job thread stays
  blocked in the call, so the run cannot go on with other files. Rejected.
- **Deadlines from the `Clock` port.** `Clock::now` is wall-clock time, which can jump,
  and it offers no way to wait. Rejected in favour of a port of its own.

## Consequences

- Every read of a source crosses a thread and copies its buffer once more.
- A stopped source leaves one blocked thread until the process ends; a long-running
  GUI holds one per source that ever hung.
- The `files` service gets `StallGuard`, `Timer`, `ManualTimer` and two faults of
  `MemoryFileSystem`; the workspace gets `adapter-timer`, and the import gets the
  timer among its ports. IMPORT-022 is built on this, then `hang-while-reading` of
  ADR 0048 §3.
