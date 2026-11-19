// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// Port for waiting with a limit in monotonic time (ADR 0049 §3).
///
/// `adapter-timer` implements it with `std::time::Instant`; tests use
/// [`testing::ManualTimer`](crate::testing::ManualTimer), whose time passes only when
/// the test advances it. Every implementation must pass
/// [`testing::check_timer`](crate::testing::check_timer).
pub trait Timer: Send + Sync {
    /// Returns the deadline `limit` from now.
    fn deadline(&self, limit: Duration) -> Deadline;

    /// Waits until `answer` is signalled or `deadline` passes, and says which came
    /// first. An answer signalled before the call wins, even if the deadline has
    /// passed too.
    fn wait(&self, answer: &AnswerSignal, deadline: Deadline) -> WaitOutcome;
}

/// A point in the monotonic time of one [`Timer`], as the time since that timer
/// started. Deadlines of different timers cannot be compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Deadline {
    since_start: Duration,
}

impl Deadline {
    /// The deadline `since_start` after the start of its timer.
    #[must_use]
    pub fn from_time_since_start(since_start: Duration) -> Self {
        Deadline { since_start }
    }

    /// The time from the start of its timer to this deadline.
    #[must_use]
    pub fn time_since_start(self) -> Duration {
        self.since_start
    }
}

/// What ended a [`Timer::wait`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitOutcome {
    /// The answer was signalled first.
    Answered,
    /// The deadline passed with no answer.
    DeadlinePassed,
}

/// The signal that an answer has arrived, which a [`Timer::wait`] waits for: a flag
/// behind a mutex and a condition variable. Clones share one flag, so one thread
/// signals what another waits for.
#[derive(Debug, Clone, Default)]
pub struct AnswerSignal {
    shared: Arc<Shared>,
}

#[derive(Debug, Default)]
struct Shared {
    answered: Mutex<bool>,
    changed: Condvar,
}

impl AnswerSignal {
    /// A signal with no answer yet.
    #[must_use]
    pub fn new() -> Self {
        AnswerSignal::default()
    }

    /// Signals the answer and wakes every wait for it.
    pub fn signal(&self) {
        *self.lock() = true;
        self.shared.changed.notify_all();
    }

    /// Whether the answer has been signalled.
    #[must_use]
    pub fn is_signalled(&self) -> bool {
        *self.lock()
    }

    /// Blocks until the answer is signalled or `timeout` has passed, and returns
    /// whether it was signalled.
    #[must_use]
    pub fn wait_for(&self, timeout: Duration) -> bool {
        let (answered, _) = self
            .shared
            .changed
            .wait_timeout_while(self.lock(), timeout, |answered| !*answered)
            .unwrap_or_else(PoisonError::into_inner);
        *answered
    }

    /// Blocks until the answer is signalled or `passed` holds, and returns whether it
    /// was signalled. `passed` is checked under the lock, so a [`Self::wake`] after a
    /// change it sees is never missed.
    pub(crate) fn wait_until(&self, passed: impl Fn() -> bool) -> bool {
        let answered = self
            .shared
            .changed
            .wait_while(self.lock(), |answered| !*answered && !passed())
            .unwrap_or_else(PoisonError::into_inner);
        *answered
    }

    /// Wakes every wait so that it checks its condition again.
    pub(crate) fn wake(&self) {
        drop(self.lock());
        self.shared.changed.notify_all();
    }

    /// Whether `self` and `other` share one flag.
    pub(crate) fn is_same_as(&self, other: &AnswerSignal) -> bool {
        Arc::ptr_eq(&self.shared, &other.shared)
    }

    fn lock(&self) -> MutexGuard<'_, bool> {
        // A panic while the flag was locked cannot leave the flag half written.
        self.shared
            .answered
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}
