// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The jobs of several sources in one `framehive import` (CLI-015): each job's reports
//! are followed on a thread of its own, and every report reaches the shared output in
//! one piece, so the lines of parallel jobs interleave only line by line.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use import::{FileReport, ImportJobReports, ImportSummary, ProgressSink, SourceScan};

use super::folders::Folders;
use super::run::{NotStarted, start_error, unknown_free_space_message};
use crate::report::ReportPrinter;
use crate::{JOB_ENDED_UNEXPECTEDLY, write_error, write_warning};

/// Standard output and standard error, shared by the threads that follow the jobs.
pub(super) struct Streams<'a> {
    output: Mutex<&'a mut (dyn Write + Send)>,
    errors: Mutex<&'a mut (dyn Write + Send)>,
}

impl<'a> Streams<'a> {
    /// Shares `output` and `errors`.
    pub(super) fn new(
        output: &'a mut (dyn Write + Send),
        errors: &'a mut (dyn Write + Send),
    ) -> Self {
        Streams {
            output: Mutex::new(output),
            errors: Mutex::new(errors),
        }
    }

    /// Writes `lines` to the output in one piece.
    pub(super) fn print(&self, lines: &[u8]) {
        if lines.is_empty() {
            return;
        }
        let mut output = self.output.lock().unwrap_or_else(PoisonError::into_inner);
        // A closed standard output must not stop the import; the exit code still tells
        // the result.
        let _ = output.write_all(lines);
        let _ = output.flush();
    }

    /// Writes each error of `result` to standard error at once, after `named_source`
    /// when there are several sources, and returns `result` (CLI-015).
    pub(super) fn reported<T>(
        &self,
        result: Result<T, NotStarted>,
        named_source: Option<&Path>,
    ) -> Result<T, NotStarted> {
        if let Err(not_started) = &result {
            let mut errors = self.errors();
            for message in &not_started.messages {
                match named_source {
                    Some(source) => {
                        write_error(&mut **errors, &format!("{}: {message}", source.display()));
                    }
                    None => write_error(&mut **errors, message),
                }
            }
        }
        result
    }

    /// Standard error, for as long as the caller holds it.
    pub(super) fn errors(&self) -> MutexGuard<'_, &'a mut (dyn Write + Send)> {
        self.errors.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A source whose job is enqueued, the printer of its report, its folders, which
/// the errors of a job that does not start name as given, and the destinations whose
/// unknown free space the command already warned about (CLI-011).
pub(super) struct Enqueued<'f> {
    pub(super) reports: ImportJobReports,
    pub(super) printer: ReportPrinter<Vec<u8>>,
    pub(super) folders: &'f Folders,
    pub(super) warned: Vec<PathBuf>,
}

impl Enqueued<'_> {
    /// Prints the job's reports as they come and, when it ends, its summary after the
    /// line `<named_source>:`, given only when there are several sources (CLI-015),
    /// then a warning for each destination whose free space only the job could not
    /// read (CLI-011.9).
    ///
    /// # Errors
    ///
    /// Returns the message of the run error when the planned run cannot start.
    pub(super) fn follow(
        self,
        named_source: Option<&Path>,
        streams: &Streams<'_>,
    ) -> Result<ImportSummary, NotStarted> {
        let mut relay = Relay {
            printer: self.printer,
            streams,
        };
        let end = self
            .reports
            .follow(&mut relay)
            .ok_or_else(|| JOB_ENDED_UNEXPECTEDLY.to_owned())?;
        // An opened camera comes back with the end report; the caller still holds it.
        let summary = end
            .result
            .map_err(|error| start_error(error, self.folders))?;
        let mut block = named_source
            .map(|source| format!("{}:\n", source.display()).into_bytes())
            .unwrap_or_default();
        relay.printer.print_summary(&summary);
        block.extend(relay.printer.take_written());
        streams.print(&block);
        let unwarned = summary
            .free_space_unknown
            .iter()
            .filter(|unknown| !self.warned.contains(&unknown.destination));
        for unknown in unwarned {
            let message = unknown_free_space_message(unknown, self.folders);
            write_warning(&mut **streams.errors(), &message);
        }
        if let Some(error) = &summary.history_error {
            write_error(&mut **streams.errors(), &error.to_string());
        }
        Ok(summary)
    }
}

/// Passes each report of one job on to the shared output as soon as it is printed.
struct Relay<'s, 'a> {
    printer: ReportPrinter<Vec<u8>>,
    streams: &'s Streams<'a>,
}

impl Relay<'_, '_> {
    fn pass_on(&mut self) {
        self.streams.print(&self.printer.take_written());
    }
}

impl ProgressSink for Relay<'_, '_> {
    fn scan_finished(&mut self, scan: &SourceScan) {
        self.printer.scan_finished(scan);
        self.pass_on();
    }

    fn file_started(&mut self, path: &Path) {
        self.printer.file_started(path);
        self.pass_on();
    }

    fn file_finished(&mut self, report: &FileReport) {
        self.printer.file_finished(report);
        self.pass_on();
    }
}
