// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import log as text to read (CLI-009).

use std::fmt::Write;
use std::path::Path;

use history::{LoggedRun, RecordedTime, RunLog};

use super::shown;

/// What a run without an end shows in place of its end.
const NOT_FINISHED: &str = "not finished";

/// The width of the labels of the header of one run.
const LABEL_WIDTH: usize = 21;

/// One line per run, oldest first, or `No import runs recorded.`.
pub(super) fn run_list(runs: &[LoggedRun]) -> String {
    if runs.is_empty() {
        return "No import runs recorded.\n".to_owned();
    }
    let mut list = String::new();
    for run in runs {
        // Writing to a String does not fail.
        let _ = writeln!(
            list,
            "{}  {}  {}  {} copies  {} -> {}",
            run.id.number(),
            run.start.started_at,
            finished(run.finished_at),
            run.copy_count,
            shown(&run.start.source),
            shown(&run.start.import_destination),
        );
    }
    list
}

/// The header of the run, then one block per destination with one line per copy.
pub(super) fn run_log(log: &RunLog) -> String {
    let run = &log.run;
    let mut text = format!("run {}\n", run.id.number());
    let header = [
        ("started", run.start.started_at.to_string()),
        ("finished", finished(run.finished_at)),
        ("source", shown(&run.start.source)),
        ("import destination", shown(&run.start.import_destination)),
    ];
    for (label, value) in header {
        // Writing to a String does not fail.
        let _ = writeln!(text, "{label:<LABEL_WIDTH$}{value}");
    }
    if log.copies.is_empty() {
        text.push_str("\nNo copies recorded.\n");
    }
    let mut destination: Option<&Path> = None;
    for copy in &log.copies {
        if destination != Some(copy.destination.as_path()) {
            destination = Some(&copy.destination);
            let _ = write!(text, "\n{}\n", shown(&copy.destination));
        }
        let _ = writeln!(
            text,
            "  {} -> {}  {}",
            shown(&copy.source_path),
            copy.target,
            copy.checksum
        );
    }
    text
}

fn finished(finished_at: Option<RecordedTime>) -> String {
    finished_at.map_or_else(|| NOT_FINISHED.to_owned(), |time| time.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_log::fixtures::{log, run};

    #[test]
    fn a_run_lists_its_numbers_times_and_folders() {
        let text = run_list(&[run(None)]);

        assert_eq!(
            text,
            "3  2026-10-31T11:00:00+02:00  not finished  2 copies  /media/card -> /library\n"
        );
    }

    #[test]
    fn the_log_of_a_run_has_a_header_and_a_block_per_destination() {
        let text = run_log(&log());

        assert_eq!(
            text,
            "run 3\n\
             started              2026-10-31T11:00:00+02:00\n\
             finished             2026-10-31T11:00:00+02:00\n\
             source               /media/card\n\
             import destination   /library\n\
             \n\
             /library\n  \
             DCIM/a,\"b\".jpg -> 2026/a,\"b\".jpg  blake3:0101\n\
             \n\
             /backup\n  \
             DCIM/a,\"b\".jpg -> 2026/a,\"b\".jpg  blake3:0101\n"
        );
    }

    #[test]
    fn a_run_without_copies_says_so() {
        let empty = RunLog {
            copies: Vec::new(),
            ..log()
        };

        assert!(run_log(&empty).ends_with("/library\n\nNo copies recorded.\n"));
    }
}
