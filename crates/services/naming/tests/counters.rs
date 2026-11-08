// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reserving ranges of the persisted sequence counters (IMPORT-002, ADR 0029).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use naming::testing::{MemorySequenceCounters, SequenceCountersUnderTest, check_sequence_counters};
use naming::{
    CounterDate, NameTemplate, SequenceCounters, SequenceCountersError, SequenceError,
    SequenceRange, StoredCounters, preview_range, reserve_range,
};

fn date(text: &str) -> CounterDate {
    CounterDate::parse(text).unwrap()
}

fn template(text: &str) -> NameTemplate {
    NameTemplate::parse(text).unwrap()
}

fn counters(stored: StoredCounters) -> MemorySequenceCounters {
    let counters = MemorySequenceCounters::new();
    counters.set(stored);
    counters
}

fn stored_at(last: u32) -> StoredCounters {
    StoredCounters {
        stored_last: last,
        ..StoredCounters::default()
    }
}

fn range(first: u32, last: u32) -> SequenceRange {
    SequenceRange { first, last }
}

/// spec: IMPORT-002.1
#[test]
fn imported_today_counter_continues_within_a_day() {
    let today = date("2026-10-17");
    let counters = counters(StoredCounters {
        imported_today_date: Some(today),
        imported_today_last: 12,
        ..StoredCounters::default()
    });
    let template = template("{date:%Y%m%d}_{seq_today:3}.{ext}");

    let reserved = reserve_range(&counters, &[&template], today, 3).unwrap();

    assert_eq!(reserved, Some(range(13, 15)));
    assert_eq!(counters.stored().imported_today_last, 15);
}

/// spec: IMPORT-002.2
#[test]
fn imported_today_counter_resets_on_a_new_day() {
    let counters = counters(StoredCounters {
        imported_today_date: Some(date("2026-10-17")),
        imported_today_last: 15,
        stored_last: 40,
    });
    let template = template("{seq_today:3}.{ext}");

    let reserved = reserve_range(&counters, &[&template], date("2026-10-18"), 2).unwrap();

    assert_eq!(reserved, Some(range(1, 2)));
    assert_eq!(
        counters.stored(),
        StoredCounters {
            imported_today_date: Some(date("2026-10-18")),
            imported_today_last: 2,
            stored_last: 40,
        }
    );
}

/// spec: IMPORT-002.4
/// spec: IMPORT-002.5
#[test]
fn a_second_import_continues_after_the_first_range() {
    let counters = counters(stored_at(100));
    let template = template("{seq_stored:4}.{ext}");
    let today = date("2026-10-17");

    let first = reserve_range(&counters, &[&template], today, 10).unwrap();
    // The first import is cancelled before copying; its numbers stay used.
    let second = reserve_range(&counters, &[&template], today, 4).unwrap();

    assert_eq!(first, Some(range(101, 110)));
    assert_eq!(second, Some(range(111, 114)));
}

/// spec: IMPORT-002.7
#[test]
fn overflow_reserves_nothing() {
    let counters = counters(stored_at(98));
    let template = template("{seq_stored:2}.{ext}");

    let error = reserve_range(&counters, &[&template], date("2026-10-17"), 5).unwrap_err();

    let SequenceError::Overflow(overflow) = error else {
        panic!("not an overflow: {error:?}");
    };
    assert_eq!(overflow.suggestion.as_deref(), Some("{seq_stored:3}.{ext}"));
    assert_eq!(counters.stored().stored_last, 98);
}

/// spec: IMPORT-002.9
#[test]
fn a_preview_reserves_nothing() {
    let counters = counters(stored_at(250));
    let template = template("{seq_stored:4}.{ext}");
    let today = date("2026-10-17");

    let previewed = preview_range(&counters, &[&template], today, 3).unwrap();
    let reserved = reserve_range(&counters, &[&template], today, 3).unwrap();

    assert_eq!(previewed, Some(range(251, 253)));
    assert_eq!(reserved, Some(range(251, 253)));
    assert_eq!(counters.stored().stored_last, 253);
}

/// spec: IMPORT-002.10
#[test]
fn broken_counters_stop_the_reservation() {
    let counters = counters(stored_at(250));
    counters.break_at("/state/framehive/sequence-counters.txt", 3);
    let template = template("{seq_stored:4}.{ext}");

    let error = reserve_range(&counters, &[&template], date("2026-10-17"), 1).unwrap_err();

    assert!(matches!(
        error,
        SequenceError::Counters(SequenceCountersError::Broken { line: 3, .. })
    ));
    assert_eq!(counters.stored().stored_last, 250);
}

/// spec: IMPORT-002.11
#[test]
fn a_template_without_a_persisted_counter_does_not_read_the_counters() {
    let counters = counters(stored_at(250));
    counters.break_at("/state/framehive/sequence-counters.txt", 3);
    let today = date("2026-10-17");

    for text in [
        "{seq_session:4}.{ext}",
        "{seq_letters}.{ext}",
        "{name}.{ext}",
    ] {
        let template = template(text);
        assert_eq!(reserve_range(&counters, &[&template], today, 2), Ok(None));
        assert_eq!(preview_range(&counters, &[&template], today, 2), Ok(None));
    }
}

#[test]
fn no_assets_reserve_no_range() {
    let counters = counters(stored_at(7));
    let template = template("{seq_stored:4}.{ext}");

    let reserved = reserve_range(&counters, &[&template], date("2026-10-17"), 0).unwrap();

    assert_eq!(reserved, None);
    assert_eq!(counters.stored().stored_last, 7);
}

struct InMemory(MemorySequenceCounters);

impl SequenceCountersUnderTest for InMemory {
    fn counters(&self) -> &dyn SequenceCounters {
        &self.0
    }

    fn reopen(&mut self) {}

    fn break_storage(&mut self) -> String {
        self.0.break_at("counters.txt", 1);
        "counters.txt".to_owned()
    }
}

#[test]
fn the_test_double_conforms_to_the_port() {
    let failures = check_sequence_counters(&mut || InMemory(MemorySequenceCounters::new()));

    assert!(failures.is_empty(), "{failures:#?}");
}
