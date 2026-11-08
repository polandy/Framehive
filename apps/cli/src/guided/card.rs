// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! One card or camera of the guided import: the loop of the plan summary, the import and
//! its report (CLI-003, CLI-006, CLI-014). Opening, reading, ejecting and releasing are
//! in `device.rs`, the summary itself in `summary.rs`.

use std::path::{Path, PathBuf};

use devices::Device;
use files::ReadOnlyFileSystem;
use import::{
    BackupOutcome, CAMERA_FORMAT_READINESS_NOTE, ConflictPolicy, FileOutcome, FileReport,
    FormatReadiness, ImportSettings, ImportSummary, JobCode, PlannedRun, RunError, SourceScan,
};
use naming::{CompatibilityMode, NameTemplate, SequenceError};

use super::dialog::{Dialog, Interrupted, Stop};
use super::error_lines::{history_line, plan_error_line, start_error_line};
use super::progress::{ImportContext, LazyProgress};
use super::source::Source;
use super::summary::Action;
use crate::JOB_ENDED_UNEXPECTEDLY;
use crate::format_readiness::format_readiness_lines;
use crate::report::{ReportPrinter, backup_line, backup_summary_lines, file_line, summary_line};

/// The settings of a guided import: compatibility mode on, `add_suffix` (CLI-003).
fn settings<'s>(
    source: &'s Source,
    destination: &'s Path,
    backups: &'s [PathBuf],
    template: &'s NameTemplate,
    job_code: Option<&'s JobCode>,
    dry_run: bool,
) -> ImportSettings<'s> {
    ImportSettings {
        source: source.root(),
        destination,
        backup_destinations: backups,
        video_destination: None,
        video_backup_destinations: &[],
        template,
        job_code,
        mode: CompatibilityMode::On,
        policy: ConflictPolicy::AddSuffix,
        dry_run,
    }
}

impl Dialog<'_> {
    /// Opens and reads `device`, asks what the dialog still needs, imports it on Enter,
    /// and offers to eject or release it; with `resumed`, imports it again at once
    /// (CLI-003, "A card pulled during the import"; CLI-014).
    pub(super) fn import_device(
        &mut self,
        device: &Device,
        resumed: Option<Interrupted>,
    ) -> Result<(), Stop> {
        let Some((source, scan)) = self.read_device(device)? else {
            return Ok(());
        };
        if scan.files.is_empty() {
            self.say(&format!(
                "{} has no photos or videos to import.",
                device.name()
            ));
            return Ok(());
        }
        let resuming = resumed.is_some();
        let mut job_code = resumed.and_then(|interrupted| interrupted.job_code);
        if self.state.template.is_none() {
            self.template_step(&source, &scan, job_code.as_ref())?;
        }
        if !resuming && self.uses_job_code() {
            job_code = Some(self.ask_job_code(device)?);
        }
        let reported = if resuming {
            self.import_at_once(&source, &scan, job_code.as_ref())
        } else {
            self.summarise(&source, &scan, job_code)?
        };
        if reported {
            self.finish_device(source)?;
        }
        Ok(())
    }

    /// Imports the device of an interrupted import again without asking; returns
    /// whether the import was reported.
    fn import_at_once(
        &mut self,
        source: &Source,
        scan: &SourceScan,
        job_code: Option<&JobCode>,
    ) -> bool {
        let Some((destination, backups)) = self.resolved_folders() else {
            return false;
        };
        let Some(template) = self.state.template.clone() else {
            return false;
        };
        let settings = settings(source, &destination, &backups, &template, job_code, false);
        let Some(plan) = self.plan(source, &settings, scan) else {
            return false;
        };
        self.import(source, &settings, plan, &[]) == Imported::Reported
    }

    /// Shows the plan summary until the person imports or quits; returns whether an
    /// import was reported.
    fn summarise(
        &mut self,
        source: &Source,
        scan: &SourceScan,
        mut job_code: Option<JobCode>,
    ) -> Result<bool, Stop> {
        let ports = self.ports;
        let run = source.run(&ports);
        let card = source.device();
        loop {
            let Some((destination, backups)) = self.resolved_folders() else {
                return Ok(false);
            };
            let Some(template) = self.state.template.clone() else {
                return Ok(false);
            };
            let settings = settings(
                source,
                &destination,
                &backups,
                &template,
                job_code.as_ref(),
                false,
            );
            let plan = match run.plan(&settings, scan) {
                Ok(plan) => plan,
                Err(RunError::Sequence(SequenceError::Overflow(overflow))) => {
                    self.say(&format!(
                        "The template cannot number {}: {overflow}",
                        card.name()
                    ));
                    self.retemplate(source, scan, &mut job_code)?;
                    continue;
                }
                Err(error) => {
                    self.say(&plan_error_line(card, &error));
                    return Ok(false);
                }
            };
            self.show_summary(card, &plan, &settings);
            let (blockers, warned) = self.blockers(&run, &settings, &plan, source.is_camera());
            for blocker in &blockers {
                self.say(blocker);
            }
            match self.summary_action(card, template.uses_job_code(), &blockers)? {
                Action::Import => match self.import(source, &settings, plan, &warned) {
                    Imported::NotStarted => {}
                    imported => return Ok(imported == Imported::Reported),
                },
                Action::List => {
                    let listing = ImportSettings {
                        dry_run: true,
                        ..settings
                    };
                    let mut printer = ReportPrinter::new(&mut *self.output, true);
                    import::ProgressSink::scan_finished(&mut printer, scan);
                    let summary = run.execute(&listing, plan, &mut printer);
                    printer.print_summary(&summary);
                }
                Action::JobCode => job_code = Some(self.ask_job_code(card)?),
                Action::Template => self.retemplate(source, scan, &mut job_code)?,
                Action::Destination => self.ask_destination(&destination)?,
                Action::Backup => self.ask_backups()?,
            }
        }
    }

    /// Runs the template step, and asks for a job code if the new template needs one.
    fn retemplate(
        &mut self,
        source: &Source,
        scan: &SourceScan,
        job_code: &mut Option<JobCode>,
    ) -> Result<(), Stop> {
        self.template_step(source, scan, job_code.as_ref())?;
        if job_code.is_none() && self.uses_job_code() {
            *job_code = Some(self.ask_job_code(source.device())?);
        }
        Ok(())
    }

    /// Plans every file of `scan`, or prints why it cannot be planned.
    fn plan(
        &mut self,
        source: &Source,
        settings: &ImportSettings<'_>,
        scan: &SourceScan,
    ) -> Option<PlannedRun> {
        let ports = self.ports;
        let run = source.run(&ports);
        match run.plan(settings, scan) {
            Ok(plan) => Some(plan),
            Err(error) => {
                self.say(&plan_error_line(source.device(), &error));
                None
            }
        }
    }

    /// Imports `plan` with the progress line, then reports with the format readiness
    /// (CLI-007). The job creates the destination and the backup destinations once its
    /// free-space check passes (IMPORT-012); a destination whose free space it cannot
    /// read and that is not in `warned` gets a warning (CLI-011.10).
    fn import(
        &mut self,
        source: &Source,
        settings: &ImportSettings<'_>,
        plan: PlannedRun,
        warned: &[PathBuf],
    ) -> Imported {
        let ports = self.ports;
        let card = source.device();
        if let Some(job_code) = settings.job_code {
            self.state.job_codes.mark_used(job_code.clone());
            self.remember();
        }
        let sizes = plan.files().iter().map(|file| file.found.size).collect();
        let (job, reports) = source.import_job(ports.import, settings);
        // The job asks before each file, so the progress polls the devices then and a
        // pulled card or camera stops it (CLI-003, CLI-014).
        let job = job.with_plan(plan).asking_before_each_file();
        if let Err(error) = job.enqueue(&mut self.jobs) {
            self.say(&start_error_line(card, &error));
            return Imported::NotStarted;
        }
        let context = ImportContext {
            output: &mut *self.output,
            watch: &mut self.watch,
            devices: ports.devices,
            cameras: ports.cameras,
            clock: &*ports.import.clock,
        };
        let camera = source
            .opened_camera()
            .map(|camera| &camera.file_system as &dyn ReadOnlyFileSystem);
        let mut progress = LazyProgress::new(context, card, camera, sizes);
        let result = match reports.follow(&mut progress) {
            Some(end) => end.result.map_err(|error| start_error_line(card, &error)),
            None => Err(format!(
                "Could not import {}: {JOB_ENDED_UNEXPECTEDLY}",
                card.name()
            )),
        };
        let summary = match result {
            Ok(summary) => summary,
            Err(line) => {
                drop(progress);
                self.say(&line);
                return Imported::NotStarted;
            }
        };
        let changes = progress.finish();
        self.show_changes(&changes);
        for unknown in &summary.free_space_unknown {
            if !warned.contains(&unknown.destination) {
                self.warn(&unknown.to_string());
            }
        }
        if summary.stopped {
            self.interrupted_by_removal(source, settings.job_code);
            return Imported::Stopped;
        }
        self.report(&summary, source);
        Imported::Reported
    }

    /// Says that the device went during its import and remembers the import, to run it
    /// again when the device is back (CLI-003, CLI-014). A camera that stopped
    /// answering may still be listed; it is reported as removed now, and not again.
    fn interrupted_by_removal(&mut self, source: &Source, job_code: Option<&JobCode>) {
        let card = source.device();
        let line = if source.is_camera() {
            if self.watch.devices().contains(card) {
                self.say(&format!("[REMOVED] {}", card.name()));
                self.forget_unanswered(card);
            }
            format!(
                "{} was disconnected during the import. Connect it again to go on.",
                card.name()
            )
        } else {
            format!(
                "{} was removed during the import. Insert it again to go on.",
                card.name()
            )
        };
        self.say(&line);
        self.interrupted = Some(Interrupted {
            name: card.name().to_owned(),
            camera: source.is_camera(),
            job_code: job_code.cloned(),
        });
    }

    /// Reports a finished import: the files that need attention, the summaries, a
    /// history that could not record, and the format readiness.
    fn report(&mut self, summary: &ImportSummary, source: &Source) {
        for report in &summary.files {
            self.report_file(report);
        }
        self.say(&summary_line(summary, false));
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

    /// Reports a file that failed or was skipped, or that failed at a backup
    /// destination, with its failed backups (CLI-003, CLI-006).
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
        if !failed && failed_backups.is_empty() {
            return;
        }
        self.say(&file_line(report));
        for backup in failed_backups {
            self.say(&backup_line(backup, &[]));
        }
    }

    /// The destination and the backup destinations as the canonical absolute paths that
    /// identify them in the import history (IMPORT-009), or `None` after saying why one
    /// cannot be resolved.
    pub(super) fn resolved_folders(&mut self) -> Option<(PathBuf, Vec<PathBuf>)> {
        let folders: Vec<PathBuf> = std::iter::once(self.destination())
            .chain(self.state.backups.iter().cloned())
            .collect();
        let mut resolved = Vec::with_capacity(folders.len());
        for folder in &folders {
            match (self.ports.resolve_folder)(folder) {
                Ok(path) => resolved.push(path),
                Err(error) => {
                    self.say(&format!("Could not use {}: {error}", folder.display()));
                    return None;
                }
            }
        }
        let backups = resolved.split_off(1);
        resolved.pop().map(|destination| (destination, backups))
    }

    /// The remembered destination, or `<home>/Pictures` before one is chosen.
    pub(super) fn destination(&self) -> PathBuf {
        self.state
            .destination
            .clone()
            .unwrap_or_else(|| self.environment.home.join("Pictures"))
    }
}

/// How an import that was asked for ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Imported {
    /// It did not start; the plan summary is shown again.
    NotStarted,
    /// The device went during the import.
    Stopped,
    /// It ran and was reported.
    Reported,
}
