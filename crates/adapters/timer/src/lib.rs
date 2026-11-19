// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `timer` adapter of Framehive: it implements the [`files::Timer`] port with
//! `std::time::Instant` and a condition variable (ADR 0049 §3). Only apps depend on
//! it. See `README.md`.
//!
//! ```
//! use std::time::Duration;
//!
//! use adapter_timer::StdTimer;
//! use files::{AnswerSignal, Timer, WaitOutcome};
//!
//! let timer = StdTimer::new();
//! let answer = AnswerSignal::new();
//! answer.signal();
//! let outcome = timer.wait(&answer, timer.deadline(Duration::from_secs(30)));
//! assert_eq!(outcome, WaitOutcome::Answered);
//! ```

#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

use files::{AnswerSignal, Deadline, Timer, WaitOutcome};

/// The [`Timer`] of the machine: monotonic time from [`Instant`], counted from when
/// the timer was created.
#[derive(Debug, Clone, Copy)]
pub struct StdTimer {
    start: Instant,
}

impl StdTimer {
    /// Creates the timer; its deadlines count from now.
    #[must_use]
    #[expect(
        clippy::disallowed_methods,
        reason = "this adapter is the Timer port's one reader of monotonic time"
    )]
    pub fn new() -> Self {
        StdTimer {
            start: Instant::now(),
        }
    }
}

impl Default for StdTimer {
    fn default() -> Self {
        StdTimer::new()
    }
}

impl Timer for StdTimer {
    fn deadline(&self, limit: Duration) -> Deadline {
        Deadline::from_time_since_start(self.start.elapsed().saturating_add(limit))
    }

    fn wait(&self, answer: &AnswerSignal, deadline: Deadline) -> WaitOutcome {
        loop {
            if answer.is_signalled() {
                return WaitOutcome::Answered;
            }
            let Some(left) = deadline
                .time_since_start()
                .checked_sub(self.start.elapsed())
                .filter(|left| !left.is_zero())
            else {
                return WaitOutcome::DeadlinePassed;
            };
            if answer.wait_for(left) {
                return WaitOutcome::Answered;
            }
        }
    }
}
