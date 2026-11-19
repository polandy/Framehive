// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`Timer`] port.

use std::thread;
use std::time::Duration;

use crate::{AnswerSignal, Timer, WaitOutcome};

/// A limit no check waits for: a check that reaches it has failed already.
const LONG: Duration = Duration::from_secs(3_600);

/// What the [`Timer`] conformance suite runs against.
pub struct TimerUnderTest<'a> {
    /// The implementation under test. The suite never lets time pass on it: it uses
    /// deadlines that have passed at once or that no check reaches.
    pub timer: &'a dyn Timer,
}

/// Runs the conformance suite of the [`Timer`] port and returns one message per
/// failed check, or nothing if the implementation conforms.
#[must_use]
pub fn check_timer(under_test: &TimerUnderTest<'_>) -> Vec<String> {
    let mut failures = Vec::new();
    check_later_limit_later_deadline(under_test.timer, &mut failures);
    check_deadline_without_answer(under_test.timer, &mut failures);
    check_answer_before_the_call(under_test.timer, &mut failures);
    check_answer_during_the_wait(under_test.timer, &mut failures);
    failures
}

fn check_later_limit_later_deadline(timer: &dyn Timer, failures: &mut Vec<String>) {
    let now = timer.deadline(Duration::ZERO).time_since_start();
    let later = timer.deadline(LONG).time_since_start();
    if later < now + LONG {
        failures.push(format!(
            "deadline: a limit of {LONG:?} gives {later:?}, before {now:?} plus the limit"
        ));
    }
}

fn check_deadline_without_answer(timer: &dyn Timer, failures: &mut Vec<String>) {
    let outcome = timer.wait(&AnswerSignal::new(), timer.deadline(Duration::ZERO));
    if outcome != WaitOutcome::DeadlinePassed {
        failures.push(format!(
            "deadline_without_answer: a passed deadline with no answer gives {outcome:?}"
        ));
    }
}

fn check_answer_before_the_call(timer: &dyn Timer, failures: &mut Vec<String>) {
    let answer = AnswerSignal::new();
    answer.signal();
    let outcome = timer.wait(&answer, timer.deadline(Duration::ZERO));
    if outcome != WaitOutcome::Answered {
        failures.push(format!(
            "answer_before_the_call: an answer signalled before a passed deadline gives \
             {outcome:?}"
        ));
    }
}

fn check_answer_during_the_wait(timer: &dyn Timer, failures: &mut Vec<String>) {
    let answer = AnswerSignal::new();
    let outcome = thread::scope(|scope| {
        let answering = answer.clone();
        scope.spawn(move || answering.signal());
        timer.wait(&answer, timer.deadline(LONG))
    });
    if outcome != WaitOutcome::Answered {
        failures.push(format!(
            "answer_during_the_wait: an answer from another thread gives {outcome:?}"
        ));
    }
}
