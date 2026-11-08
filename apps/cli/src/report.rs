// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The report of `framehive import` (CLI-001, CLI-005, CLI-017): one line per file as
//! soon as it is done, each followed by its sidecar, value and backup lines, then the
//! summary and the format readiness (CLI-007, in `format_readiness`).

use std::io::Write;
use std::path::{Path, PathBuf};

use import::{
    BackupError, BackupOutcome, BackupReport, ExistingValue, ExistingValueResult, FileError,
    FileOutcome, FileReport, FormatReadiness, ImportSummary, LeftOut, ProgressSink, SourceScan,
};

use crate::format_readiness::format_readiness_lines;

/// A backup destination as the import run knows it, and as the report names it.
#[derive(Debug, Clone)]
pub(crate) struct BackupName {
    /// The resolved path, as in the settings of the run.
    pub(crate) path: PathBuf,
    /// The folder as the person gave it (CLI-005).
    pub(crate) shown: PathBuf,
}

/// A [`ProgressSink`] that writes the report lines to `output` as the run goes.
pub(crate) struct ReportPrinter<W: Write> {
    output: W,
    dry_run: bool,
    backup_names: Vec<BackupName>,
    /// A note printed after the format readiness, such as the one of a camera run
    /// (IMPORT-014.10).
    format_readiness_note: Option<&'static str>,
}

impl<W: Write> ReportPrinter<W> {
    /// Creates a printer; `dry_run` turns `copied` into `would copy`.
    pub(crate) fn new(output: W, dry_run: bool) -> Self {
        ReportPrinter {
            output,
            dry_run,
            backup_names: Vec::new(),
            format_readiness_note: None,
        }
    }

    /// Names the backup destinations in the backup lines as `names` say.
    pub(crate) fn with_backup_names(mut self, names: Vec<BackupName>) -> Self {
        self.backup_names = names;
        self
    }

    /// Prints `note` after the format readiness of a run that wrote.
    pub(crate) fn with_format_readiness_note(mut self, note: &'static str) -> Self {
        self.format_readiness_note = Some(note);
        self
    }

    /// Writes the summary line of a finished run, and after a run that wrote, the
    /// line of each backup destination and the format readiness of the source.
    pub(crate) fn print_summary(&mut self, summary: &ImportSummary) {
        let line = summary_line(summary, self.dry_run);
        self.write_line(&line);
        if self.dry_run {
            return;
        }
        if let Some(line) = sidecars_written_line(summary) {
            self.write_line(&line);
        }
        for line in backup_summary_lines(summary, &self.backup_names) {
            self.write_line(&line);
        }
        for line in format_readiness_lines(&FormatReadiness::of(summary), "source") {
            self.write_line(&line);
        }
        if let Some(note) = self.format_readiness_note {
            self.write_line(note);
        }
    }

    fn write_line(&mut self, line: &str) {
        // A closed standard output must not stop the import; the exit code still
        // tells the result.
        let _ = writeln!(self.output, "{line}");
        let _ = self.output.flush();
    }
}

impl ReportPrinter<Vec<u8>> {
    /// Takes the lines written so far, so they can be passed on as one piece.
    pub(crate) fn take_written(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.output)
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
        for line in sidecar_lines(report, self.dry_run) {
            self.write_line(&line);
        }
        for value in &report.existing_values {
            self.write_line(&existing_value_line(value));
        }
        for backup in &report.backups {
            let line = backup_line(backup, &self.backup_names);
            self.write_line(&line);
        }
    }
}

/// The sidecars a file got, each as `<target>` or `<source> -> <target>` (CLI-017).
fn written_sidecars(report: &FileReport) -> Vec<String> {
    if matches!(report.outcome, FileOutcome::Failed(_)) {
        return Vec::new();
    }
    let would_copy = matches!(report.outcome, FileOutcome::WouldCopy { .. });
    let mut sidecars = Vec::new();
    let mut values_carried = false;
    for carried in &report.carried_sidecars {
        let Some(target) = &carried.target else {
            continue;
        };
        let holds_values = report.sidecar.as_ref() == Some(target);
        values_carried |= holds_values;
        if holds_values || carried.verified_at > 0 || would_copy {
            sidecars.push(format!("{} -> {target}", carried.path.display()));
        }
    }
    if !values_carried && let Some(target) = &report.sidecar {
        sidecars.push(target.clone());
    }
    sidecars
}

/// The indented line of each sidecar a file got (CLI-017).
pub(crate) fn sidecar_lines(report: &FileReport, dry_run: bool) -> Vec<String> {
    let verb = if dry_run { "would write" } else { "wrote" };
    written_sidecars(report)
        .into_iter()
        .map(|sidecar| format!("  {verb} sidecar  {sidecar}"))
        .collect()
}

/// The indented line of a value the carried-along sidecar already had (CLI-017).
pub(crate) fn existing_value_line(value: &ExistingValue) -> String {
    match value.result {
        ExistingValueResult::Replaced => format!(
            "  replaced {}  {} -> {}",
            value.field, value.existing, value.import
        ),
        ExistingValueResult::Kept => format!(
            "  kept {}  {} (not {})",
            value.field, value.existing, value.import
        ),
    }
}

/// The line after the summary line of a run that wrote sidecars (CLI-017).
pub(crate) fn sidecars_written_line(summary: &ImportSummary) -> Option<String> {
    let written: usize = summary
        .files
        .iter()
        .map(|report| written_sidecars(report).len())
        .sum();
    (written > 0).then(|| format!("{written} sidecars written"))
}

/// The folder a backup destination is named by in the report: as given, if known.
fn shown<'n>(path: &'n Path, names: &'n [BackupName]) -> std::path::Display<'n> {
    names
        .iter()
        .find(|name| name.path == path)
        .map_or(path, |name| name.shown.as_path())
        .display()
}

/// The indented line of one backup of a file (CLI-005, "Report").
pub(crate) fn backup_line(backup: &BackupReport, names: &[BackupName]) -> String {
    let folder = shown(&backup.destination, names);
    match &backup.outcome {
        BackupOutcome::BackedUp => format!("  backed up  {folder}"),
        BackupOutcome::AlreadyBackedUp => format!("  already backed up  {folder}"),
        BackupOutcome::BackedUpBefore => format!("  backed up before  {folder}"),
        BackupOutcome::Failed(error) => {
            format!(
                "  backup failed  {folder}: {}",
                backup_failure_reason(error)
            )
        }
    }
}

/// The reason of a failed backup; a name conflict reads as for a failed file.
fn backup_failure_reason(error: &BackupError) -> String {
    match error {
        BackupError::NameConflict { .. } => "name conflict".to_owned(),
        error => error.to_string(),
    }
}

/// The line of each backup destination after the summary line (CLI-005).
pub(crate) fn backup_summary_lines(summary: &ImportSummary, names: &[BackupName]) -> Vec<String> {
    summary
        .backups
        .iter()
        .map(|counts| {
            format!(
                "backup {}: {} backed up, {} already backed up, {} failed",
                shown(&counts.destination, names),
                counts.backed_up,
                // Backed up before counts as already backed up (CLI-008).
                counts.already_backed_up + counts.backed_up_before,
                counts.failed
            )
        })
        .collect()
}

/// Whether a file failed at a backup destination, which makes the exit code 1
/// (CLI-005); `ImportSummary::has_failures` counts only the import destination.
pub(crate) fn has_backup_failures(summary: &ImportSummary) -> bool {
    summary.backups.iter().any(|counts| counts.failed > 0)
}

/// The line for a file the scan left out.
pub(crate) fn left_out_line(left_out: &LeftOut) -> String {
    format!("left out  {}: {}", left_out.path.display(), left_out.reason)
}

/// The line for a file the run is done with; the line of a file whose target is below
/// the video destination ends with `  (video destination)` (CLI-012).
pub(crate) fn file_line(report: &FileReport) -> String {
    let line = outcome_line(report);
    if report.in_video_destination {
        format!("{line}  (video destination)")
    } else {
        line
    }
}

/// The line for a file the run is done with, without the mark of CLI-012.
fn outcome_line(report: &FileReport) -> String {
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
        FileOutcome::ImportedBefore { target } => {
            format!("imported before  {source} -> {target}")
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
            // Imported before counts as already imported (CLI-008).
            FileOutcome::AlreadyImported { .. } | FileOutcome::ImportedBefore { .. } => {
                &mut counts.already_imported
            }
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
            backups: Vec::new(),
            in_video_destination: false,
            required_locations: 1,
            sidecar: None,
            carried_sidecars: Vec::new(),
            existing_values: Vec::new(),
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
            stopped: false,
            dry_run: true,
            history_error: None,
            backups: Vec::new(),
            free_space_unknown: Vec::new(),
        };

        assert_eq!(
            summary_line(&summary, true),
            "1 would copy, 0 already imported, 1 duplicates, 0 skipped, 1 failed, 1 left out"
        );
    }
}
