// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! One card of the guided import: reading it, the plan summary, the import and the
//! eject (CLI-003).

use std::path::{Path, PathBuf};

use devices::{Device, EjectError};
use domain::LocalDateTime;
use import::{
    ConflictPolicy, FileOutcome, ImportRun, ImportSettings, JobCode, PlannedRun, SourceScan,
    scan_folder,
};
use naming::{CompatibilityMode, NameTemplate};

use super::dialog::{Dialog, Interrupted, Stop};
use super::format::size_text;
use super::progress::{ImportContext, ImportProgress};
use super::state::create_folders;
use crate::report::{ReportPrinter, file_line, summary_line};

/// The line under a summary whose destination lies inside the card.
const ON_CARD: &str = "The destination is on the card; type d to choose another.";

/// What the person chose at the plan summary.
enum Action {
    Import,
    List,
    JobCode,
    Template,
    Destination,
}

/// The settings of a guided import: compatibility mode on, `add_suffix` (CLI-003).
fn settings<'s>(
    card: &'s Device,
    destination: &'s Path,
    template: &'s NameTemplate,
    job_code: Option<&'s JobCode>,
    dry_run: bool,
) -> ImportSettings<'s> {
    ImportSettings {
        source: card.mount_point(),
        destination,
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
            let Some(template) = self.state.template.clone() else {
                return Ok(());
            };
            let settings = settings(card, &destination, &template, job_code.as_ref(), false);
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
        let run = ImportRun::new(ports.file_system, ports.hasher, ports.clock);
        loop {
            let destination = self.destination();
            let Some(template) = self.state.template.clone() else {
                return Ok(());
            };
            let settings = settings(card, &destination, &template, job_code.as_ref(), false);
            let Some(plan) = self.plan(card, &settings, scan) else {
                return Ok(());
            };
            self.show_summary(card, &plan, &settings);
            let on_card = destination.starts_with(card.mount_point());
            if on_card {
                self.say(ON_CARD);
            }
            match self.summary_action(card, template.uses_job_code(), on_card)? {
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
                Action::Template => {
                    self.template_step(card, scan, job_code.as_ref())?;
                    if job_code.is_none() && self.uses_job_code() {
                        job_code = Some(self.ask_job_code(card)?);
                    }
                }
                Action::Destination => self.ask_destination(&destination)?,
            }
        }
    }

    /// Plans every file of `scan`, or prints why it cannot be planned.
    fn plan(
        &mut self,
        card: &Device,
        settings: &ImportSettings<'_>,
        scan: &SourceScan,
    ) -> Option<PlannedRun> {
        let ports = self.ports;
        let run = ImportRun::new(ports.file_system, ports.hasher, ports.clock);
        match run.plan(settings, scan) {
            Ok(plan) => Some(plan),
            Err(error) => {
                self.say(&format!("Could not plan {}: {error}", card.name()));
                None
            }
        }
    }

    /// Prints the plan summary (CLI-003, "Plan summary").
    fn show_summary(&mut self, card: &Device, plan: &PlannedRun, settings: &ImportSettings<'_>) {
        let files = plan.files();
        let bytes = files.iter().map(|file| file.found.size).sum();
        let captured = captured_text(plan)
            .map(|dates| format!(", captured {dates}"))
            .unwrap_or_default();
        self.say(&format!(
            "{}: {} files, {}{captured}",
            card.name(),
            files.len(),
            size_text(bytes)
        ));
        self.say(&format!("  to        {}", settings.destination.display()));
        self.say(&format!("  named     {}", settings.template.as_str()));
        if settings.template.uses_job_code()
            && let Some(job_code) = settings.job_code
        {
            self.say(&format!("  job code  {}", job_code.as_str()));
        }
    }

    /// Asks what to do at the plan summary.
    fn summary_action(
        &mut self,
        card: &Device,
        uses_job_code: bool,
        on_card: bool,
    ) -> Result<Action, Stop> {
        let (prompt, hint) = if uses_job_code {
            (
                "[Enter] import  [l] list  [j] job code  [t] template  [d] destination  [q] quit: ",
                "Type l, j, t, d or q, or press Enter to import.",
            )
        } else {
            (
                "[Enter] import  [l] list  [t] template  [d] destination  [q] quit: ",
                "Type l, t, d or q, or press Enter to import.",
            )
        };
        loop {
            match self.prompt(prompt, Some(card))?.as_str() {
                "" if on_card => self.say(ON_CARD),
                "" => return Ok(Action::Import),
                "l" => return Ok(Action::List),
                "j" if uses_job_code => return Ok(Action::JobCode),
                "t" => return Ok(Action::Template),
                "d" => return Ok(Action::Destination),
                _ => self.say(hint),
            }
        }
    }

    /// Creates the destination and imports `plan` with the progress line, then reports
    /// and offers the eject. Returns `false` when the import could not start.
    fn import(
        &mut self,
        card: &Device,
        settings: &ImportSettings<'_>,
        plan: PlannedRun,
    ) -> Result<bool, Stop> {
        let ports = self.ports;
        if let Err(error) = create_folders(ports.file_system, settings.destination) {
            self.say(&format!(
                "Could not create {}: {error}",
                settings.destination.display()
            ));
            return Ok(false);
        }
        if let Some(job_code) = settings.job_code {
            self.state.job_codes.mark_used(job_code.clone());
            self.remember();
        }
        let sizes = plan.files().iter().map(|file| file.found.size).collect();
        let context = ImportContext {
            output: &mut *self.output,
            watch: &mut self.watch,
            devices: ports.devices,
            clock: ports.clock,
        };
        let mut progress = ImportProgress::start(context, card, sizes);
        let run = ImportRun::new(ports.file_system, ports.hasher, ports.clock);
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
            if matches!(
                report.outcome,
                FileOutcome::Failed(_) | FileOutcome::SkippedNameConflict { .. }
            ) {
                self.say(&file_line(report));
            }
        }
        self.say(&summary_line(&summary, false));
        self.offer_eject(card)?;
        Ok(true)
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

/// `2026-10-17 to 2026-10-19`, or one date, for the planned files with a date.
fn captured_text(plan: &PlannedRun) -> Option<String> {
    let dates = plan
        .files()
        .iter()
        .filter_map(|file| file.planned.as_ref().ok())
        .map(|planned| planned.date);
    let first = dates.clone().min()?;
    let last = dates.max()?;
    if (first.year(), first.month(), first.day()) == (last.year(), last.month(), last.day()) {
        Some(date_text(first))
    } else {
        Some(format!("{} to {}", date_text(first), date_text(last)))
    }
}

fn date_text(time: LocalDateTime) -> String {
    format!("{:04}-{:02}-{:02}", time.year(), time.month(), time.day())
}
