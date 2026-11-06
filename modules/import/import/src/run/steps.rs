// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Where an executing run hands its steps: to a [`ProgressSink`] that only looks at
//! each file report, or to an import job that sends it on (IMPORT-015).

use std::path::Path;

use crate::report::{FileReport, ImportSummary, ProgressSink};
use crate::scan::SourceScan;

/// The steps of a run, with each file report handed over by value. A file report
/// holds errors and cannot be copied, so it goes to exactly one owner.
pub(crate) trait RunSteps {
    /// The source was scanned.
    fn scan_finished(&mut self, scan: &SourceScan);
    /// The run starts the file at `path`, relative to the source.
    fn file_started(&mut self, path: &Path);
    /// The file is done.
    fn file_finished(&mut self, report: FileReport);
    /// Asked before each file: whether the run goes on.
    fn should_continue(&mut self) -> bool;
}

/// Shows each step to a [`ProgressSink`] and keeps the file reports for the summary.
pub(super) struct Keeping<'p> {
    progress: &'p mut dyn ProgressSink,
    files: Vec<FileReport>,
}

impl<'p> Keeping<'p> {
    pub(super) fn new(progress: &'p mut dyn ProgressSink) -> Self {
        Keeping {
            progress,
            files: Vec::new(),
        }
    }

    /// `summary` with the file reports kept.
    pub(super) fn into_summary(self, summary: ImportSummary) -> ImportSummary {
        ImportSummary {
            files: self.files,
            ..summary
        }
    }
}

impl RunSteps for Keeping<'_> {
    fn scan_finished(&mut self, scan: &SourceScan) {
        self.progress.scan_finished(scan);
    }

    fn file_started(&mut self, path: &Path) {
        self.progress.file_started(path);
    }

    fn file_finished(&mut self, report: FileReport) {
        self.progress.file_finished(&report);
        self.files.push(report);
    }

    fn should_continue(&mut self) -> bool {
        self.progress.should_continue()
    }
}
