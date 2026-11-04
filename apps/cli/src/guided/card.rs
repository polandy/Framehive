// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! One card of the guided import: reading it, the loop of the plan summary, the import
//! and the eject (CLI-003, CLI-006). The summary itself is in `summary.rs`.

use std::path::{Path, PathBuf};

use devices::{Device, EjectError};
use import::{
    BackupOutcome, ConflictPolicy, FileOutcome, FileReport, ImportSettings, JobCode, PlannedRun,
    RunError, SourceScan, scan_folder,
};
use naming::{CompatibilityMode, NameTemplate, SequenceError};

use super::dialog::{Dialog, Interrupted, Stop};
use super::progress::{ImportContext, ImportProgress};
use super::state::create_folders;
use super::summary::{Action, blockers};
use crate::report::{ReportPrinter, backup_line, backup_summary_lines, file_line, summary_line};

/// The settings of a guided import: compatibility mode on, `add_suffix` (CLI-003).
fn settings<'s>(
    card: &'s Device,
    destination: &'s Path,
    backups: &'s [PathBuf],
    template: &'s NameTemplate,
    job_code: Option<&'s JobCode>,
    dry_run: bool,
) -> ImportSettings<'s> {
    ImportSettings {
        source: card.mount_point(),
        destination,
        backup_destinations: backups,
        template,
        job_code,
        mode: CompatibilityMode::On,
        policy: ConflictPolicy::AddSuffix,
        dry_run,
    }
}

impl Dialog<'_> {
    /// Reads `card`, asks what the dialog still needs, and imports it on Enter; with
    /// `resumed`, imports it again at once (CLI-003, "A card pulled during the import").
    pub(super) fn import_card(
        &mut self,
        card: &Device,
        resumed: Option<Interrupted>,
    ) -> Result<(), Stop> {
        self.say(&format!("Reading {}…", card.name()));
        let scan = match scan_folder(self.ports.file_system, card.mount_point()) {
            Ok(scan) => scan,
            Err(error) => {
                self.say(&format!("Could not read {}: {error}", card.name()));
                return Ok(());
            }
        };
        if scan.files.is_empty() {
            self.say(&format!(
                "{} has no photos or videos to import.",
                card.name()
            ));
            return Ok(());
        }
        let resuming = resumed.is_some();
        let mut job_code = resumed.and_then(|interrupted| interrupted.job_code);
        if self.state.template.is_none() {
            self.template_step(card, &scan, job_code.as_ref())?;
        }
        if !resuming && self.uses_job_code() {
            job_code = Some(self.ask_job_code(card)?);
        }
        if resuming {
            let destination = self.destination();
            let backups = self.state.backups.clone();
            let Some(template) = self.state.template.clone() else {
                return Ok(());
            };
            let settings = settings(
                card,
                &destination,
                &backups,
                &template,
                job_code.as_ref(),
                false,
            );
            if let Some(plan) = self.plan(card, &settings, &scan) {
                self.import(card, &settings, plan)?;
            }
            return Ok(());
        }
        self.summarise(card, &scan, job_code)
    }

    /// Shows the plan summary until the person imports or quits.
    fn summarise(
        &mut self,
        card: &Device,
        scan: &SourceScan,
        mut job_code: Option<JobCode>,
    ) -> Result<(), Stop> {
        let ports = self.ports;
        let run = ports.import_run();
        loop {
            let destination = self.destination();
            let backups = self.state.backups.clone();
            let Some(template) = self.state.template.clone() else {
                return Ok(());
            };
            let settings = settings(
                card,
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
                    self.retemplate(card, scan, &mut job_code)?;
                    continue;
                }
                Err(error) => {
                    self.say(&format!("Could not plan {}: {error}", card.name()));
                    return Ok(());
                }
            };
            self.show_summary(card, &plan, &settings);
            let blockers = blockers(&settings);
            for blocker in &blockers {
                self.say(blocker);
            }
            match self.summary_action(card, template.uses_job_code(), &blockers)? {
                Action::Import => {
                    if self.import(card, &settings, plan)? {
                        return Ok(());
                    }
                }
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
                Action::Template => self.retemplate(card, scan, &mut job_code)?,
                Action::Destination => self.ask_destination(&destination)?,
                Action::Backup => self.ask_backups()?,
            }
        }
    }

    /// Runs the template step, and asks for a job code if the new template needs one.
    fn retemplate(
        &mut self,
        card: &Device,
        scan: &SourceScan,
        job_code: &mut Option<JobCode>,
    ) -> Result<(), Stop> {
        self.template_step(card, scan, job_code.as_ref())?;
        if job_code.is_none() && self.uses_job_code() {
            *job_code = Some(self.ask_job_code(card)?);
        }
        Ok(())
    }

    /// Plans every file of `scan`, or prints why it cannot be planned.
    fn plan(
        &mut self,
        card: &Device,
        settings: &ImportSettings<'_>,
        scan: &SourceScan,
    ) -> Option<PlannedRun> {
        let ports = self.ports;
        let run = ports.import_run();
        match run.plan(settings, scan) {
            Ok(plan) => Some(plan),
            Err(error) => {
                self.say(&format!("Could not plan {}: {error}", card.name()));
                None
            }
        }
    }

    /// Creates the destination and the backup destinations and imports `plan` with the
    /// progress line, then reports and offers the eject. Returns `false` when the import
    /// could not start.
    fn import(
        &mut self,
        card: &Device,
        settings: &ImportSettings<'_>,
        plan: PlannedRun,
    ) -> Result<bool, Stop> {
        let ports = self.ports;
        let folders = std::iter::once(settings.destination)
            .chain(settings.backup_destinations.iter().map(PathBuf::as_path));
        for folder in folders {
            if let Err(error) = create_folders(ports.file_system, folder) {
                self.say(&format!("Could not create {}: {error}", folder.display()));
                return Ok(false);
            }
        }
        if let Some(job_code) = settings.job_code {
            self.state.job_codes.mark_used(job_code.clone());
            self.remember();
        }
        let run = ports.import_run();
        let plan = match run.start(settings, plan) {
            Ok(plan) => plan,
            Err(error) => {
                self.say(&format!("Could not import {}: {error}", card.name()));
                return Ok(false);
            }
        };
        let sizes = plan.files().iter().map(|file| file.found.size).collect();
        let context = ImportContext {
            output: &mut *self.output,
            watch: &mut self.watch,
            devices: ports.devices,
            clock: ports.clock,
        };
        let mut progress = ImportProgress::start(context, card, sizes);
        let summary = run.execute(settings, plan, &mut progress);
        let changes = progress.finish();
        self.show_changes(&changes);
        if summary.stopped {
            self.say(&format!(
                "{} was removed during the import. Insert it again to go on.",
                card.name()
            ));
            self.interrupted = Some(Interrupted {
                name: card.name().to_owned(),
                job_code: settings.job_code.cloned(),
            });
            return Ok(true);
        }
        for report in &summary.files {
            self.report_file(report);
        }
        self.say(&summary_line(&summary, false));
        for line in backup_summary_lines(&summary, &[]) {
            self.say(&line);
        }
        self.offer_eject(card)?;
        Ok(true)
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

    /// Asks whether to eject `card` and ejects it (CLI-003, "Report and eject").
    fn offer_eject(&mut self, card: &Device) -> Result<(), Stop> {
        if !self.watch.devices().contains(card) {
            return Ok(());
        }
        let prompt = format!("Eject {}? [Y/n]: ", card.name());
        if !self.ask_yes(&prompt, Some(card))? {
            return Ok(());
        }
        match self.ports.devices.eject(card) {
            Ok(()) => {
                self.say(&format!("{} can be removed now.", card.name()));
                self.forget_ejected(card);
            }
            Err(error) => {
                let reason = match error {
                    EjectError::Failed { message, .. } => message,
                    error => error.to_string(),
                };
                self.say(&format!("Could not eject {}: {reason}", card.name()));
            }
        }
        Ok(())
    }

    /// Whether the remembered template has `{job}`.
    pub(super) fn uses_job_code(&self) -> bool {
        self.state
            .template
            .as_ref()
            .is_some_and(NameTemplate::uses_job_code)
    }

    /// The remembered destination, or `<home>/Pictures` before one is chosen.
    pub(super) fn destination(&self) -> PathBuf {
        self.state
            .destination
            .clone()
            .unwrap_or_else(|| self.environment.home.join("Pictures"))
    }
}
