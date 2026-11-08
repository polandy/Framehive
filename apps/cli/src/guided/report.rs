// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The report of a finished guided import (CLI-003, CLI-006, CLI-007, CLI-017).

use import::{
    BackupOutcome, CAMERA_FORMAT_READINESS_NOTE, FileOutcome, FileReport, FormatReadiness,
    ImportSummary,
};

use super::dialog::Dialog;
use super::error_lines::history_line;
use super::source::Source;
use crate::format_readiness::format_readiness_lines;
use crate::report::{
    backup_line, backup_summary_lines, existing_value_line, file_line, sidecars_written_line,
    summary_line,
};

impl Dialog<'_> {
    /// Reports a finished import: the files that need attention, the summaries, a
    /// history that could not record, and the format readiness.
    pub(super) fn report(&mut self, summary: &ImportSummary, source: &Source) {
        for report in &summary.files {
            self.report_file(report);
        }
        self.say(&summary_line(summary, false));
        if let Some(line) = sidecars_written_line(summary) {
            self.say(&line);
        }
        for line in backup_summary_lines(summary, &[]) {
            self.say(&line);
        }
        if let Some(error) = &summary.history_error {
            self.say(&history_line(error));
        }
        // A card pulled meanwhile was imported again as a whole (CLI-003.8), so this
        // summary covers every file of the card (CLI-007.10).
        let lines = format_readiness_lines(&FormatReadiness::of(summary), source.noun());
        for line in &lines {
            self.say(line);
        }
        if source.is_camera() && !lines.is_empty() {
            self.say(CAMERA_FORMAT_READINESS_NOTE);
        }
    }

    /// Reports a file that failed or was skipped, that failed at a backup destination,
    /// or whose sidecar had values of its own, with those values and its failed backups
    /// (CLI-003, CLI-006, CLI-017).
    fn report_file(&mut self, report: &FileReport) {
        let failed_backups: Vec<_> = report
            .backups
            .iter()
            .filter(|backup| matches!(backup.outcome, BackupOutcome::Failed(_)))
            .collect();
        let failed = matches!(
            report.outcome,
            FileOutcome::Failed(_) | FileOutcome::SkippedNameConflict { .. }
        );
        if !failed && failed_backups.is_empty() && report.existing_values.is_empty() {
            return;
        }
        self.say(&file_line(report));
        for value in &report.existing_values {
            self.say(&existing_value_line(value));
        }
        for backup in failed_backups {
            self.say(&backup_line(backup, &[]));
        }
    }
}
