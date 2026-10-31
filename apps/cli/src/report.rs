// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The report of `framehive import` (CLI-001): one line per file as soon as it is
//! done, then the summary.

use std::io::Write;
use std::path::Path;

use import::{
    FileError, FileOutcome, FileReport, ImportSummary, LeftOut, ProgressSink, SourceScan,
};

/// A [`ProgressSink`] that writes the report lines to `output` as the run goes.
pub(crate) struct ReportPrinter<W: Write> {
    output: W,
    dry_run: bool,
}

impl<W: Write> ReportPrinter<W> {
    /// Creates a printer; `dry_run` turns `copied` into `would copy`.
    pub(crate) fn new(output: W, dry_run: bool) -> Self {
        ReportPrinter { output, dry_run }
    }

    /// Writes the summary line of a finished run.
    pub(crate) fn print_summary(&mut self, summary: &ImportSummary) {
        let line = summary_line(summary, self.dry_run);
        self.write_line(&line);
    }

    fn write_line(&mut self, line: &str) {
        // A closed standard output must not stop the import; the exit code still
        // tells the result.
        let _ = writeln!(self.output, "{line}");
        let _ = self.output.flush();
    }
}

impl<W: Write> ProgressSink for ReportPrinter<W> {
    fn scan_finished(&mut self, scan: &SourceScan) {
        for left_out in &scan.left_out {
            self.write_line(&left_out_line(left_out));
        }
    }

    fn file_started(&mut self, _path: &Path) {}

    fn file_finished(&mut self, report: &FileReport) {
        self.write_line(&file_line(report));
    }
}

/// The line for a file the scan left out.
pub(crate) fn left_out_line(left_out: &LeftOut) -> String {
    format!("left out  {}: {}", left_out.path.display(), left_out.reason)
}

/// The line for a file the run is done with.
pub(crate) fn file_line(report: &FileReport) -> String {
    let source = report.path.display();
    let dated = report
        .dated_by_modification_time
        .as_ref()
        .map(|reason| format!(" (dated by modification time: {reason})"))
        .unwrap_or_default();
    match &report.outcome {
        FileOutcome::Copied { target, .. } => format!("copied  {source} -> {target}{dated}"),
        FileOutcome::WouldCopy { target } => format!("would copy  {source} -> {target}{dated}"),
        FileOutcome::AlreadyImported { target } => {
            format!("already imported  {source} -> {target}")
        }
        FileOutcome::DuplicateOf { earlier } => {
            format!("duplicate  {source} = {}", earlier.display())
        }
        FileOutcome::SkippedNameConflict { .. } => format!("skipped  {source}: name conflict"),
        FileOutcome::Failed(error) => format!("failed  {source}: {}", failure_reason(error)),
    }
}

/// The reason of a failed file; a name conflict reads as under `skip` (CLI-001.4).
fn failure_reason(error: &FileError) -> String {
    match error {
        FileError::NameConflict { .. } => "name conflict".to_owned(),
        error => error.to_string(),
    }
}

/// Counts for the summary line.
#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    copied: usize,
    already_imported: usize,
    duplicates: usize,
    skipped: usize,
    failed: usize,
}

fn count(summary: &ImportSummary) -> Counts {
    let mut counts = Counts::default();
    for report in &summary.files {
        let counter = match report.outcome {
            FileOutcome::Copied { .. } | FileOutcome::WouldCopy { .. } => &mut counts.copied,
            FileOutcome::AlreadyImported { .. } => &mut counts.already_imported,
            FileOutcome::DuplicateOf { .. } => &mut counts.duplicates,
            FileOutcome::SkippedNameConflict { .. } => &mut counts.skipped,
            FileOutcome::Failed(_) => &mut counts.failed,
        };
        *counter += 1;
    }
    counts
}

/// The summary line; in a dry run, `copied` reads `would copy`.
pub(crate) fn summary_line(summary: &ImportSummary, dry_run: bool) -> String {
    let counts = count(summary);
    let copied = if dry_run { "would copy" } else { "copied" };
    format!(
        "{} {copied}, {} already imported, {} duplicates, {} skipped, {} failed, {} left out",
        counts.copied,
        counts.already_imported,
        counts.duplicates,
        counts.skipped,
        counts.failed,
        summary.left_out.len()
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use import::LeftOutReason;

    use super::*;

    fn report(path: &str, outcome: FileOutcome) -> FileReport {
        FileReport {
            path: PathBuf::from(path),
            dated_by_modification_time: None,
            outcome,
        }
    }

    #[test]
    fn a_duplicate_names_the_earlier_file() {
        let duplicate = report(
            "DCIM/101/DSCF0001.RAF",
            FileOutcome::DuplicateOf {
                earlier: PathBuf::from("DCIM/100/DSCF0001.RAF"),
            },
        );

        assert_eq!(
            file_line(&duplicate),
            "duplicate  DCIM/101/DSCF0001.RAF = DCIM/100/DSCF0001.RAF"
        );
    }

    #[test]
    fn the_summary_counts_each_outcome_and_what_was_left_out() {
        let summary = ImportSummary {
            files: vec![
                report(
                    "a.jpg",
                    FileOutcome::WouldCopy {
                        target: "a.jpg".to_owned(),
                    },
                ),
                report(
                    "b.jpg",
                    FileOutcome::DuplicateOf {
                        earlier: PathBuf::from("a.jpg"),
                    },
                ),
                report("c.jpg", FileOutcome::Failed(FileError::NotPlanned)),
            ],
            left_out: vec![LeftOut {
                path: PathBuf::from("notes.txt"),
                reason: LeftOutReason::UnsupportedType,
            }],
        };

        assert_eq!(
            summary_line(&summary, true),
            "1 would copy, 0 already imported, 1 duplicates, 0 skipped, 1 failed, 1 left out"
        );
    }
}
