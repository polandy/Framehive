// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The checks of the conformance suite for the import log (IMPORT-010):
//! [`ImportHistory::list_runs`] and [`ImportHistory::read_run_log`].

use std::path::{Path, PathBuf};

use super::ImportHistoryUnderTest;
use super::conformance::{BACKUP, DESTINATION, copy, now, record, start};
use crate::{CopyRecord, ImportHistory, LoggedRun, RunId, RunLog, RunStart};

/// Runs the import log checks, each on a fresh environment from `fresh`.
pub(super) fn check_import_log<T: ImportHistoryUnderTest>(
    fresh: &mut dyn FnMut() -> T,
    failures: &mut Vec<String>,
) {
    check_no_runs(&fresh(), failures);
    check_runs_oldest_first(&fresh(), failures);
    check_run_without_end(&fresh(), failures);
    check_copies_of_a_run(&fresh(), failures);
    check_copy_without_capture_time(&fresh(), failures);
    check_only_own_copies(&fresh(), failures);
    check_missing_run(&fresh(), failures);
    check_log_paths_are_bytes(&fresh(), failures);
}

fn list(history: &dyn ImportHistory, check: &str, failures: &mut Vec<String>) -> Vec<LoggedRun> {
    history.list_runs().unwrap_or_else(|error| {
        failures.push(format!("{check}: listing the runs fails: {error}"));
        Vec::new()
    })
}

fn read_log(
    history: &dyn ImportHistory,
    run: RunId,
    check: &str,
    failures: &mut Vec<String>,
) -> Option<RunLog> {
    match history.read_run_log(run) {
        Ok(Some(log)) => Some(log),
        Ok(None) => {
            failures.push(format!("{check}: the log of {run:?} is missing"));
            None
        }
        Err(error) => {
            failures.push(format!(
                "{check}: reading the log of {run:?} fails: {error}"
            ));
            None
        }
    }
}

fn finish(history: &dyn ImportHistory, run: RunId, check: &str, failures: &mut Vec<String>) {
    if let Err(error) = history.finish_run(run, now()) {
        failures.push(format!("{check}: finishing {run:?} fails: {error}"));
    }
}

/// The start [`start`] records.
fn run_start() -> RunStart {
    RunStart {
        source: PathBuf::from("/media/card"),
        import_destination: PathBuf::from(DESTINATION),
        started_at: now(),
    }
}

/// A copy of `name` at `destination`, as [`copy`] makes it.
fn named(run: RunId, destination: &str, name: &str, byte: u8) -> CopyRecord {
    CopyRecord {
        source_path: PathBuf::from(format!("DCIM/100_FUJI/{name}")),
        original_file_name: name.to_owned(),
        ..copy(run, Path::new(destination), byte, &format!("2026/{name}"))
    }
}

fn check_no_runs(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    let runs = list(under_test.history(), "no_runs", failures);
    if !runs.is_empty() {
        failures.push(format!("no_runs: an empty history lists {runs:?}"));
    }
}

fn check_runs_oldest_first(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    const CHECK: &str = "runs_oldest_first";
    let history = under_test.history();
    let Some(first) = start(history, CHECK, failures) else {
        return;
    };
    record(
        history,
        &named(first, DESTINATION, "DSCF0001.RAF", 1),
        CHECK,
        failures,
    );
    finish(history, first, CHECK, failures);
    let second_start = RunStart {
        source: PathBuf::from("/media/other-card"),
        ..run_start()
    };
    let second = match history.start_run(&second_start) {
        Ok(second) => second,
        Err(error) => {
            failures.push(format!("{CHECK}: starting a run fails: {error}"));
            return;
        }
    };
    record(
        history,
        &named(second, DESTINATION, "DSCF0002.RAF", 2),
        CHECK,
        failures,
    );
    record(
        history,
        &named(second, BACKUP, "DSCF0002.RAF", 2),
        CHECK,
        failures,
    );
    finish(history, second, CHECK, failures);

    let runs = list(history, CHECK, failures);
    let expected = [
        LoggedRun {
            id: first,
            start: run_start(),
            finished_at: Some(now()),
            copy_count: 1,
        },
        LoggedRun {
            id: second,
            start: second_start,
            finished_at: Some(now()),
            copy_count: 2,
        },
    ];
    if runs != expected {
        failures.push(format!("{CHECK}: {runs:?} is listed, not {expected:?}"));
    }
}

fn check_run_without_end(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    const CHECK: &str = "run_without_end";
    let history = under_test.history();
    if start(history, CHECK, failures).is_none() {
        return;
    }
    let runs = list(history, CHECK, failures);
    if runs.len() != 1 || runs.iter().any(|run| run.finished_at.is_some()) {
        failures.push(format!(
            "{CHECK}: {runs:?} is listed for one run without an end"
        ));
    }
}

fn check_copies_of_a_run(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    const CHECK: &str = "copies_of_a_run";
    let history = under_test.history();
    let Some(run) = start(history, CHECK, failures) else {
        return;
    };
    let first = named(run, DESTINATION, "DSCF0001.RAF", 1);
    let backed_up = named(run, BACKUP, "DSCF0001.RAF", 1);
    let second = named(run, DESTINATION, "DSCF0002.RAF", 2);
    for copy in [&first, &backed_up, &second] {
        record(history, copy, CHECK, failures);
    }
    let Some(log) = read_log(history, run, CHECK, failures) else {
        return;
    };
    let expected = [first, second, backed_up];
    if log.copies != expected {
        failures.push(format!(
            "{CHECK}: the log holds {:?}, not {expected:?}",
            log.copies
        ));
    }
    let expected_run = LoggedRun {
        id: run,
        start: run_start(),
        finished_at: None,
        copy_count: 3,
    };
    if log.run != expected_run {
        failures.push(format!(
            "{CHECK}: the log is of {:?}, not {expected_run:?}",
            log.run
        ));
    }
}

fn check_copy_without_capture_time(
    under_test: &dyn ImportHistoryUnderTest,
    failures: &mut Vec<String>,
) {
    const CHECK: &str = "copy_without_capture_time";
    let history = under_test.history();
    let Some(run) = start(history, CHECK, failures) else {
        return;
    };
    let notes = CopyRecord {
        capture_time: None,
        ..named(run, DESTINATION, "notes.png", 3)
    };
    record(history, &notes, CHECK, failures);
    let Some(log) = read_log(history, run, CHECK, failures) else {
        return;
    };
    if log.copies != [notes] {
        failures.push(format!("{CHECK}: the log holds {:?}", log.copies));
    }
}

fn check_only_own_copies(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    const CHECK: &str = "only_own_copies";
    let history = under_test.history();
    let (Some(first), Some(second)) = (
        start(history, CHECK, failures),
        start(history, CHECK, failures),
    ) else {
        return;
    };
    let copied = named(first, DESTINATION, "DSCF0001.RAF", 1);
    record(history, &copied, CHECK, failures);
    if let Some(log) = read_log(history, second, CHECK, failures)
        && !log.copies.is_empty()
    {
        failures.push(format!(
            "{CHECK}: the second run's log holds {:?}",
            log.copies
        ));
    }
    if let Some(log) = read_log(history, first, CHECK, failures)
        && log.copies != [copied]
    {
        failures.push(format!(
            "{CHECK}: the first run's log holds {:?}",
            log.copies
        ));
    }
}

fn check_missing_run(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    const CHECK: &str = "missing_run";
    let history = under_test.history();
    let Some(run) = start(history, CHECK, failures) else {
        return;
    };
    let missing = RunId::new(run.number() + 7);
    match history.read_run_log(missing) {
        Ok(None) => {}
        Ok(Some(log)) => failures.push(format!("{CHECK}: {missing:?} has the log {log:?}")),
        Err(error) => failures.push(format!("{CHECK}: reading {missing:?} fails: {error}")),
    }
}

/// Paths that are not valid UTF-8 come back as the same bytes (ADR 0031, section 3).
#[cfg(unix)]
fn check_log_paths_are_bytes(under_test: &dyn ImportHistoryUnderTest, failures: &mut Vec<String>) {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    const CHECK: &str = "log_paths_are_bytes";
    let history = under_test.history();
    let run_start = RunStart {
        source: PathBuf::from(OsStr::from_bytes(b"/media/\xfd")),
        import_destination: PathBuf::from(OsStr::from_bytes(b"/library-\xff")),
        started_at: now(),
    };
    let run = match history.start_run(&run_start) {
        Ok(run) => run,
        Err(error) => {
            failures.push(format!("{CHECK}: starting a run fails: {error}"));
            return;
        }
    };
    let recorded = CopyRecord {
        destination: run_start.import_destination.clone(),
        source_path: PathBuf::from(OsStr::from_bytes(b"DCIM/\xfe/DSCF0001.RAF")),
        ..named(run, DESTINATION, "DSCF0001.RAF", 1)
    };
    record(history, &recorded, CHECK, failures);
    let Some(log) = read_log(history, run, CHECK, failures) else {
        return;
    };
    if log.run.start != run_start || log.copies != [recorded] {
        failures.push(format!("{CHECK}: the log {log:?} changed the paths"));
    }
}

#[cfg(not(unix))]
fn check_log_paths_are_bytes(
    _under_test: &dyn ImportHistoryUnderTest,
    _failures: &mut Vec<String>,
) {
}
