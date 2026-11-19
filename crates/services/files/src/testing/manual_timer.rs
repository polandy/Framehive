// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use crate::{AnswerSignal, Deadline, Timer, WaitOutcome};

/// A [`Timer`] whose time passes only when the test advances it, like
/// `domain::testing::ManualClock` (ADR 0049 §3).
///
/// ```
/// use std::time::Duration;
///
/// use files::testing::ManualTimer;
/// use files::{AnswerSignal, Timer, WaitOutcome};
///
/// let timer = ManualTimer::new();
/// let deadline = timer.deadline(Duration::from_secs(30));
/// timer.advance(Duration::from_secs(30));
/// assert_eq!(timer.wait(&AnswerSignal::new(), deadline), WaitOutcome::DeadlinePassed);
/// ```
#[derive(Debug, Default)]
pub struct ManualTimer {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    since_start: Duration,
    waiting_for: Vec<AnswerSignal>,
}

impl ManualTimer {
    /// A timer at its start.
    #[must_use]
    pub fn new() -> Self {
        ManualTimer::default()
    }

    /// Lets `by` pass and wakes every wait whose deadline has passed.
    pub fn advance(&self, by: Duration) {
        let waiting_for = {
            let mut state = self.lock();
            state.since_start = state.since_start.saturating_add(by);
            state.waiting_for.clone()
        };
        // Woken outside the lock of the timer: a wait holds its signal's lock while
        // it reads the time.
        for answer in waiting_for {
            answer.wake();
        }
    }

    /// The time passed since the start.
    #[must_use]
    pub fn time_since_start(&self) -> Duration {
        self.lock().since_start
    }

    /// How many waits are blocked right now, so a test can advance the time once a
    /// wait has begun.
    #[must_use]
    pub fn waits(&self) -> usize {
        self.lock().waiting_for.len()
    }

    fn has_passed(&self, deadline: Deadline) -> bool {
        self.time_since_start() >= deadline.time_since_start()
    }

    fn stop_waiting_for(&self, answer: &AnswerSignal) {
        let mut state = self.lock();
        if let Some(index) = state.waiting_for.iter().position(|w| w.is_same_as(answer)) {
            state.waiting_for.swap_remove(index);
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Timer for ManualTimer {
    fn deadline(&self, limit: Duration) -> Deadline {
        Deadline::from_time_since_start(self.time_since_start().saturating_add(limit))
    }

    fn wait(&self, answer: &AnswerSignal, deadline: Deadline) -> WaitOutcome {
        self.lock().waiting_for.push(answer.clone());
        let answered = answer.wait_until(|| self.has_passed(deadline));
        self.stop_waiting_for(answer);
        if answered {
            WaitOutcome::Answered
        } else {
            WaitOutcome::DeadlinePassed
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use super::*;
    use crate::testing::{TimerUnderTest, check_timer};

    const LIMIT: Duration = Duration::from_secs(30);

    fn wait_in_thread(
        timer: &Arc<ManualTimer>,
        answer: &AnswerSignal,
        deadline: Deadline,
    ) -> thread::JoinHandle<WaitOutcome> {
        let (waiting, answer) = (Arc::clone(timer), answer.clone());
        let handle = thread::spawn(move || waiting.wait(&answer, deadline));
        while timer.waits() == 0 {
            thread::yield_now();
        }
        handle
    }

    #[test]
    fn conforms_to_the_port() {
        let failures = check_timer(&TimerUnderTest {
            timer: &ManualTimer::new(),
        });

        assert_eq!(failures, Vec::<String>::new());
    }

    #[test]
    fn a_deadline_lies_the_limit_after_the_time_passed() {
        let timer = ManualTimer::new();
        timer.advance(Duration::from_secs(5));

        let deadline = timer.deadline(LIMIT);

        assert_eq!(deadline.time_since_start(), Duration::from_secs(35));
    }

    #[test]
    fn advancing_to_the_deadline_ends_a_blocked_wait() {
        let timer = Arc::new(ManualTimer::new());
        let deadline = timer.deadline(LIMIT);
        let waiting = wait_in_thread(&timer, &AnswerSignal::new(), deadline);

        timer.advance(LIMIT);

        assert_eq!(waiting.join().unwrap(), WaitOutcome::DeadlinePassed);
        assert_eq!(timer.waits(), 0);
    }

    #[test]
    fn advancing_short_of_the_deadline_keeps_the_wait_blocked() {
        let timer = Arc::new(ManualTimer::new());
        let answer = AnswerSignal::new();
        let deadline = timer.deadline(LIMIT);
        let waiting = wait_in_thread(&timer, &answer, deadline);

        timer.advance(Duration::from_secs(29));
        answer.signal();

        assert_eq!(waiting.join().unwrap(), WaitOutcome::Answered);
    }

    #[test]
    fn an_answer_ends_a_blocked_wait_without_advancing() {
        let timer = Arc::new(ManualTimer::new());
        let answer = AnswerSignal::new();
        let waiting = wait_in_thread(&timer, &answer, timer.deadline(LIMIT));

        answer.signal();

        assert_eq!(waiting.join().unwrap(), WaitOutcome::Answered);
        assert_eq!(timer.time_since_start(), Duration::ZERO);
    }
}
