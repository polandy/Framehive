// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The counter file on disk: what is written, broken files, and imports that reserve
//! at the same time (IMPORT-002, ADR 0029).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::thread;

use adapter_sequence_counters::FileSequenceCounters;
use naming::{
    CounterDate, NameTemplate, SequenceCounters, SequenceCountersError, SequenceError,
    SequenceRange, StoredCounters, reserve_range,
};
use support::TempDir;

fn today() -> CounterDate {
    CounterDate::parse("2026-10-17").unwrap()
}

#[test]
fn the_file_has_the_documented_format() {
    let folder = TempDir::new();
    let counters = FileSequenceCounters::new(folder.path().to_path_buf());
    let stored = StoredCounters {
        imported_today_date: Some(today()),
        imported_today_last: 15,
        stored_last: 250,
    };

    counters.update(&mut |_| Some(stored)).unwrap();

    assert_eq!(
        fs::read_to_string(counters.file()).unwrap(),
        "# Framehive keeps its sequence counters here (IMPORT-002).\n\
         format_version=1\n\
         imported_today_date=2026-10-17\n\
         imported_today_last=15\n\
         stored_last=250\n"
    );
    assert!(!folder.path().join("sequence-counters.txt.part").exists());
}

/// spec: IMPORT-002.10
#[test]
fn a_broken_file_names_its_line_and_stays_unchanged() {
    let folder = TempDir::new();
    let counters = FileSequenceCounters::new(folder.path().to_path_buf());
    let text = "format_version=1\nimported_today_last=3\nstored_last=many\n";
    fs::write(counters.file(), text).unwrap();
    let template = NameTemplate::parse("{seq_stored:4}.{ext}").unwrap();

    let error = reserve_range(&counters, &template, today(), 1).unwrap_err();

    let SequenceError::Counters(error @ SequenceCountersError::Broken { line: 3, .. }) = error
    else {
        panic!("not a broken file at line 3: {error:?}");
    };
    assert!(
        error
            .to_string()
            .contains(&counters.file().display().to_string())
    );
    assert_eq!(fs::read_to_string(counters.file()).unwrap(), text);
}

/// spec: IMPORT-002.4
#[test]
fn imports_that_reserve_at_the_same_time_never_share_a_number() {
    let folder = TempDir::new();
    let template = NameTemplate::parse("{seq_stored:6}.{ext}").unwrap();
    FileSequenceCounters::new(folder.path().to_path_buf())
        .update(&mut |stored| {
            Some(StoredCounters {
                stored_last: 100,
                ..stored
            })
        })
        .unwrap();

    // Each thread opens the lock file itself, so the lock works as between processes.
    let ranges: Vec<SequenceRange> = thread::scope(|scope| {
        let workers: Vec<_> = (1..=8_u32)
            .map(|worker| {
                let (template, folder) = (&template, folder.path());
                scope.spawn(move || {
                    let counters = FileSequenceCounters::new(folder.to_path_buf());
                    (0..10)
                        .map(|_| {
                            reserve_range(&counters, template, today(), worker)
                                .unwrap()
                                .unwrap()
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap())
            .collect()
    });

    let mut numbers: Vec<u32> = ranges
        .iter()
        .flat_map(|range| range.first..=range.last)
        .collect();
    numbers.sort_unstable();
    let total: u32 = (1..=8).sum::<u32>() * 10;
    assert_eq!(numbers, (101..=100 + total).collect::<Vec<_>>());
}
