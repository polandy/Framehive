// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`ImportHistory`] port (ADR 0031).

use std::path::{Path, PathBuf};

use domain::{Checksum, LocalDateTime, Timestamp, UtcOffset};

use crate::{CopyKey, CopyRecord, FoundCopy, HistoryError, ImportHistory, RecordedTime, RunStart};

/// An import history and the storage behind it, for [`check_import_history`].
///
/// Each check starts from a fresh one with nothing stored.
pub trait ImportHistoryUnderTest {
    /// The implementation under test.
    fn history(&self) -> &dyn ImportHistory;

    /// Makes a new implementation over the same storage, as a restart does.
    fn reopen(&mut self);

    /// Breaks the storage so that it cannot be used, and returns the text the error
    /// must name, such as the path of the database file.
    fn break_storage(&mut self) -> String;

    /// Whether the storage is still exactly as [`ImportHistoryUnderTest::break_storage`]
    /// left it.
    fn is_unchanged_since_broken(&self) -> bool;
}

/// Runs the conformance suite of the [`ImportHistory`] port and returns one message
/// per failed check, or nothing if the implementation conforms.
///
/// `fresh` makes a new environment with nothing stored for each check.
#[must_use]
pub fn check_import_history<T: ImportHistoryUnderTest>(
    fresh: &mut dyn FnMut() -> T,
) -> Vec<String> {
    let mut failures = Vec::new();
    check_starts_empty(&fresh(), &mut failures);
    check_copy_is_found(&fresh(), &mut failures);
    check_key_filters(&fresh(), &mut failures);
    check_oldest_first(&fresh(), &mut failures);
    check_survives_reopen(&mut fresh(), &mut failures);
    check_runs_are_numbered(&fresh(), &mut failures);
    check_paths_are_bytes(&fresh(), &mut failures);
    check_broken_storage(&mut fresh(), &mut failures);
    failures
}

const DESTINATION: &str = "/library";
const BACKUP: &str = "/backup";

// The fixtures are fixed, valid values; a failure is a bug of this suite.
#[allow(clippy::expect_used, reason = "fixed, valid fixture values")]
fn now() -> RecordedTime {
    let offset = UtcOffset::from_seconds(7_200).expect("a valid offset");
    RecordedTime::at(Timestamp::from_unix_seconds(1_793_437_200), offset).expect("a valid time")
}

fn captured() -> Option<LocalDateTime> {
    LocalDateTime::new(2026, 10, 17, 14, 5, 9).ok()
}

#[allow(clippy::expect_used, reason = "fixed, valid fixture values")]
fn checksum(byte: u8) -> Checksum {
    Checksum::new(Checksum::BLAKE3, vec![byte; 32]).expect("a non-empty digest")
}

fn start(
    history: &dyn ImportHistory,
    check: &str,
    failures: &mut Vec<String>,
) -> Option<crate::RunId> {
    let run = RunStart {
        source: PathBuf::from("/media/card"),
        import_destination: PathBuf::from(DESTINATION),
        started_at: now(),
    };
    match history.start_run(&run) {
        Ok(id) => Some(id),
        Err(error) => {
            failures.push(format!("{check}: starting a run fails: {error}"));
            None
        }
    }
}

/// A copy of `DSCF0001.RAF`, 10 bytes, captured 2026-10-17 14:05:09, at `destination`.
fn copy(run: crate::RunId, destination: &Path, byte: u8, target: &str) -> CopyRecord {
    CopyRecord {
        run,
        destination: destination.to_path_buf(),
        source_path: PathBuf::from("DCIM/100_FUJI/DSCF0001.RAF"),
        original_file_name: "DSCF0001.RAF".to_owned(),
        size: 10,
        capture_time: captured(),
        checksum: checksum(byte),
        target: target.to_owned(),
        recorded_at: now(),
    }
}

fn key(destination: &Path) -> CopyKey<'_> {
    CopyKey {
        destination,
        original_file_name: "DSCF0001.RAF",
        size: 10,
        capture_time: captured(),
    }
}

fn record(history: &dyn ImportHistory, copy: &CopyRecord, check: &str, failures: &mut Vec<String>) {
    if let Err(error) = history.record_copy(copy) {
        failures.push(format!("{check}: recording a copy fails: {error}"));
    }
}

fn find(
    history: &dyn ImportHistory,
    key: &CopyKey<'_>,
    check: &str,
    failures: &mut Vec<String>,
) -> Vec<FoundCopy> {
    history.find_copies(key).unwrap_or_else(|error| {
        failures.push(format!("{check}: finding copies fails: {error}"));
        Vec::new()
    })
}

fn found(copy: &CopyRecord) -> FoundCopy {
    FoundCopy {
        target: copy.target.clone(),
        checksum: copy.checksum.clone(),
    }
}

fn check_starts_empty(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    let copies = find(
        under_test.history(),
        &key(Path::new(DESTINATION)),
        "starts_empty",
        failures,
    );
    if !copies.is_empty() {
        failures.push(format!("starts_empty: an empty history finds {copies:?}"));
    }
}

fn check_copy_is_found(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    let history = under_test.history();
    let Some(run) = start(history, "copy_is_found", failures) else {
        return;
    };
    let recorded = copy(run, Path::new(DESTINATION), 1, "2026-10-17/DSCF0001.RAF");
    record(history, &recorded, "copy_is_found", failures);
    let copies = find(
        history,
        &key(Path::new(DESTINATION)),
        "copy_is_found",
        failures,
    );
    if copies != [found(&recorded)] {
        failures.push(format!(
            "copy_is_found: {copies:?} is found after recording {recorded:?}"
        ));
    }
}

fn check_key_filters(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    let history = under_test.history();
    let Some(run) = start(history, "key_filters", failures) else {
        return;
    };
    let destination = Path::new(DESTINATION);
    let with_time = copy(run, destination, 1, "a/DSCF0001.RAF");
    let without_time = CopyRecord {
        capture_time: None,
        ..copy(run, destination, 2, "b/DSCF0001.RAF")
    };
    record(history, &with_time, "key_filters", failures);
    record(history, &without_time, "key_filters", failures);
    for (case, key, expected) in key_cases(destination, &with_time, &without_time) {
        let copies = find(history, &key, "key_filters", failures);
        if copies != expected {
            failures.push(format!(
                "key_filters: {case} finds {copies:?}, not {expected:?}"
            ));
        }
    }
}

/// Each key of [`check_key_filters`] and what it finds.
fn key_cases<'a>(
    destination: &'a Path,
    with_time: &CopyRecord,
    without_time: &CopyRecord,
) -> [(&'static str, CopyKey<'a>, Vec<FoundCopy>); 6] {
    let other_time = LocalDateTime::new(2026, 10, 17, 15, 5, 9).ok();
    [
        (
            "the same capture time",
            key(destination),
            vec![found(with_time)],
        ),
        (
            "any capture time",
            CopyKey {
                capture_time: None,
                ..key(destination)
            },
            vec![found(with_time), found(without_time)],
        ),
        ("another destination", key(Path::new(BACKUP)), Vec::new()),
        (
            "another file name",
            CopyKey {
                original_file_name: "DSCF0002.RAF",
                ..key(destination)
            },
            Vec::new(),
        ),
        (
            "another size",
            CopyKey {
                size: 11,
                ..key(destination)
            },
            Vec::new(),
        ),
        (
            "another capture time",
            CopyKey {
                capture_time: other_time,
                ..key(destination)
            },
            Vec::new(),
        ),
    ]
}

fn check_oldest_first(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    let history = under_test.history();
    let Some(run) = start(history, "oldest_first", failures) else {
        return;
    };
    let destination = Path::new(DESTINATION);
    let copies: Vec<CopyRecord> = ["c", "a", "b"]
        .iter()
        .map(|folder| copy(run, destination, 1, &format!("{folder}/DSCF0001.RAF")))
        .collect();
    for recorded in &copies {
        record(history, recorded, "oldest_first", failures);
    }
    let expected: Vec<FoundCopy> = copies.iter().map(found).collect();
    let found = find(history, &key(destination), "oldest_first", failures);
    if found != expected {
        failures.push(format!(
            "oldest_first: {found:?} is found, not {expected:?}"
        ));
    }
}

fn check_survives_reopen(under_test: &mut dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    let Some(run) = start(under_test.history(), "survives_reopen", failures) else {
        return;
    };
    let recorded = copy(run, Path::new(DESTINATION), 1, "DSCF0001.RAF");
    record(under_test.history(), &recorded, "survives_reopen", failures);
    under_test.reopen();
    if let Err(error) = under_test.history().finish_run(run, now()) {
        failures.push(format!(
            "survives_reopen: finishing the run after a restart fails: {error}"
        ));
    }
    let copies = find(
        under_test.history(),
        &key(Path::new(DESTINATION)),
        "survives_reopen",
        failures,
    );
    if copies != [found(&recorded)] {
        failures.push(format!(
            "survives_reopen: {copies:?} is found after a restart"
        ));
    }
}

fn check_runs_are_numbered(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    let history = under_test.history();
    let first = start(history, "runs_are_numbered", failures);
    let second = start(history, "runs_are_numbered", failures);
    if let (Some(first), Some(second)) = (first, second) {
        if first == second {
            failures.push(format!(
                "runs_are_numbered: two runs get the same id {first:?}"
            ));
        }
        for run in [first, second] {
            if let Err(error) = history.finish_run(run, now()) {
                failures.push(format!(
                    "runs_are_numbered: finishing {run:?} fails: {error}"
                ));
            }
        }
    }
}

/// A folder path that is not valid UTF-8 is stored as its bytes (ADR 0031, section 3).
#[cfg(unix)]
fn check_paths_are_bytes(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let history = under_test.history();
    let Some(run) = start(history, "paths_are_bytes", failures) else {
        return;
    };
    let destination = Path::new(OsStr::from_bytes(b"/library-\xff"));
    let recorded = CopyRecord {
        source_path: PathBuf::from(OsStr::from_bytes(b"DCIM/\xfe/DSCF0001.RAF")),
        ..copy(run, destination, 1, "DSCF0001.RAF")
    };
    record(history, &recorded, "paths_are_bytes", failures);
    let copies = find(history, &key(destination), "paths_are_bytes", failures);
    if copies != [found(&recorded)] {
        failures.push(format!(
            "paths_are_bytes: {copies:?} is found at a non-UTF-8 path"
        ));
    }
    let lossy = PathBuf::from(destination.to_string_lossy().into_owned());
    let copies = find(history, &key(&lossy), "paths_are_bytes", failures);
    if !copies.is_empty() {
        failures.push(format!(
            "paths_are_bytes: {copies:?} is found at the lossy path"
        ));
    }
}

#[cfg(not(unix))]
fn check_paths_are_bytes(_under_test: &dyn ImportHistoryUnderTest, _failures: &mut Vec<String>) {}

fn check_broken_storage(under_test: &mut dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    let named = under_test.break_storage();
    let names_storage = |result: Result<(), HistoryError>| match result {
        Err(error) => error.to_string().contains(&named),
        Ok(()) => false,
    };
    let history = under_test.history();
    let run_start = RunStart {
        source: PathBuf::from("/media/card"),
        import_destination: PathBuf::from(DESTINATION),
        started_at: now(),
    };
    let results = [
        (
            "finding copies",
            history.find_copies(&key(Path::new(DESTINATION))).map(drop),
        ),
        ("starting a run", history.start_run(&run_start).map(drop)),
        (
            "recording a copy",
            history.record_copy(&copy(
                crate::RunId::new(1),
                Path::new(DESTINATION),
                1,
                "x.RAF",
            )),
        ),
    ];
    for (step, result) in results {
        if !names_storage(result) {
            failures.push(format!(
                "broken_storage: {step} does not fail with an error that names {named}"
            ));
        }
    }
    if !under_test.is_unchanged_since_broken() {
        failures.push("broken_storage: the broken storage was changed".to_owned());
    }
}
