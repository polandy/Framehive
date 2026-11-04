// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`SequenceCounters`] port (ADR 0029).

use crate::{CounterDate, SequenceCounters, SequenceCountersError, StoredCounters};

/// Sequence counters and the storage behind them, for [`check_sequence_counters`].
///
/// Each check starts from a fresh one with nothing stored.
pub trait SequenceCountersUnderTest {
    /// The implementation under test.
    fn counters(&self) -> &dyn SequenceCounters;

    /// Makes a new implementation over the same storage, as a restart does.
    fn reopen(&mut self);

    /// Breaks the stored values so that they cannot be understood, and returns the
    /// text the error must name, such as the path of the counter file.
    fn break_storage(&mut self) -> String;
}

/// Runs the conformance suite of the [`SequenceCounters`] port and returns one message
/// per failed check, or nothing if the implementation conforms.
///
/// `fresh` makes a new environment with nothing stored for each check.
#[must_use]
pub fn check_sequence_counters<T: SequenceCountersUnderTest>(
    fresh: &mut dyn FnMut() -> T,
) -> Vec<String> {
    let mut failures = Vec::new();
    check_starts_at_zero(&mut fresh(), &mut failures);
    check_update_is_stored(&mut fresh(), &mut failures);
    check_update_survives_reopen(&mut fresh(), &mut failures);
    check_declined_update(&mut fresh(), &mut failures);
    check_broken_storage(&mut fresh(), &mut failures);
    failures
}

/// Values in every field, with the largest number a counter may hold.
fn sample() -> StoredCounters {
    StoredCounters {
        imported_today_date: CounterDate::parse("2026-10-17"),
        imported_today_last: 15,
        stored_last: 999_999,
    }
}

fn read(
    under_test: &dyn SequenceCountersUnderTest,
    check: &str,
    failures: &mut Vec<String>,
) -> Option<StoredCounters> {
    match under_test.counters().read() {
        Ok(stored) => Some(stored),
        Err(error) => {
            failures.push(format!("{check}: reading fails: {error}"));
            None
        }
    }
}

fn store(
    under_test: &dyn SequenceCountersUnderTest,
    values: StoredCounters,
    check: &str,
    failures: &mut Vec<String>,
) {
    if let Err(error) = under_test.counters().update(&mut |_| Some(values)) {
        failures.push(format!("{check}: updating fails: {error}"));
    }
}

fn check_starts_at_zero(
    under_test: &mut dyn SequenceCountersUnderTest,
    failures: &mut Vec<String>,
) {
    let stored = read(under_test, "starts_at_zero", failures);
    if stored.is_some_and(|stored| stored != StoredCounters::default()) {
        failures.push(format!(
            "starts_at_zero: nothing stored reads as {stored:?}"
        ));
    }
}

fn check_update_is_stored(
    under_test: &mut dyn SequenceCountersUnderTest,
    failures: &mut Vec<String>,
) {
    store(under_test, sample(), "update_is_stored", failures);
    let mut given = None;
    let result = under_test.counters().update(&mut |stored| {
        given = Some(stored);
        None
    });
    if given != Some(sample()) || result != Ok(sample()) {
        failures.push(format!(
            "update_is_stored: the next update is given {given:?} and returns {result:?}"
        ));
    }
    let stored = read(under_test, "update_is_stored", failures);
    if stored.is_some_and(|stored| stored != sample()) {
        failures.push(format!(
            "update_is_stored: {stored:?} is read after storing {:?}",
            sample()
        ));
    }
}

fn check_update_survives_reopen(
    under_test: &mut dyn SequenceCountersUnderTest,
    failures: &mut Vec<String>,
) {
    store(under_test, sample(), "update_survives_reopen", failures);
    under_test.reopen();
    let stored = read(under_test, "update_survives_reopen", failures);
    if stored.is_some_and(|stored| stored != sample()) {
        failures.push(format!(
            "update_survives_reopen: {stored:?} is read after a restart"
        ));
    }
}

fn check_declined_update(
    under_test: &mut dyn SequenceCountersUnderTest,
    failures: &mut Vec<String>,
) {
    store(under_test, sample(), "declined_update", failures);
    if let Err(error) = under_test.counters().update(&mut |_| None) {
        failures.push(format!(
            "declined_update: an update that stores nothing fails: {error}"
        ));
    }
    let stored = read(under_test, "declined_update", failures);
    if stored.is_some_and(|stored| stored != sample()) {
        failures.push(format!("declined_update: the values changed to {stored:?}"));
    }
}

fn check_broken_storage(
    under_test: &mut dyn SequenceCountersUnderTest,
    failures: &mut Vec<String>,
) {
    let named = under_test.break_storage();
    let names_storage = |result: &Result<StoredCounters, SequenceCountersError>| match result {
        Err(error @ SequenceCountersError::Broken { .. }) => error.to_string().contains(&named),
        _ => false,
    };
    let read = under_test.counters().read();
    if !names_storage(&read) {
        failures.push(format!(
            "broken_storage: reading gives {read:?}, not an error naming {named:?}"
        ));
    }
    let mut called = false;
    let updated = under_test.counters().update(&mut |_| {
        called = true;
        Some(sample())
    });
    if !names_storage(&updated) || called {
        failures.push(format!(
            "broken_storage: updating gives {updated:?} and calls the change: {called}"
        ));
    }
}
