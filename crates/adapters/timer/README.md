<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-timer

Layer: `adapter`, port `files` (architecture §3)

## Purpose

Implements the `Timer` port of `files` with `std::time::Instant` and the condition
variable of `files::AnswerSignal` (ADR 0049 §3). The import will use it to wait for
each call to a source with the stall limit of the profile (IMPORT-022).

- **Monotonic time.** A deadline is the time since the timer was created, read with
  `Instant`, so a change of the wall clock never moves it. It is the only crate that
  reads `Instant::now`; `clippy.toml` forbids it everywhere else.
- **Waiting.** `wait` blocks on the signal with `Condvar::wait_timeout` for the time
  left and checks again after each wake-up, so a spurious wake-up neither ends the
  wait early nor loses an answer.
- **Not covered here:** what is waited for and what a passed deadline means
  (`StallGuard` of `files`, IMPORT-022).

## Public API

| Item | Meaning |
|---|---|
| `StdTimer` | `files::Timer` over `std::time::Instant`; `StdTimer::new()` or `Default` |

## Conformance

`tests/conformance.rs` runs `files::testing::check_timer`. The suite lets no time
pass: it uses deadlines that have passed at once or that no check reaches.

## Dependencies

- `files`
