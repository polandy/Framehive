// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The template step and the job code prompt of the guided import (CLI-002).

use devices::Device;
use import::{ImportRun, JobCode, PlannedRun, SourceScan};
use naming::{NameTemplate, TemplateError};

use super::dialog::{Dialog, Stop};

/// The template presets of the template step: label and template (CLI-002).
pub(super) const PRESETS: [(&str, &str); 3] = [
    ("By day:", "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}"),
    (
        "By day and job code:",
        "{date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}",
    ),
    (
        "By month, renamed:",
        "{date:%Y}/{date:%Y-%m}/{date:%Y%m%d}_{number:4}.{ext}",
    ),
];

/// How many files the preview of a template shows.
const PREVIEW_FILES: usize = 5;

/// The job code a preview uses before one is chosen.
const PREVIEW_JOB_CODE: &str = "job-code";

impl Dialog<'_> {
    /// Asks for a template until the person accepts one in the preview, and remembers
    /// it (CLI-002, "Template step").
    pub(super) fn template_step(
        &mut self,
        card: &Device,
        scan: &SourceScan,
        job_code: Option<&JobCode>,
    ) -> Result<(), Stop> {
        loop {
            let template = self.ask_template(card)?;
            if self.preview(card, scan, &template, job_code)? {
                self.state.template = Some(template);
                self.remember();
                return Ok(());
            }
        }
    }

    /// Shows the presets and returns the first answer that parses.
    fn ask_template(&mut self, card: &Device) -> Result<NameTemplate, Stop> {
        loop {
            self.say("How should the photos be named?");
            for (index, (label, template)) in PRESETS.iter().enumerate() {
                self.say(&format!("  {}  {label:<21}{template}", index + 1));
            }
            let default = self.remembered_preset();
            let prompt = format!(
                "Choose 1–{} or type a template [{default}]: ",
                PRESETS.len()
            );
            let answer = self.prompt(&prompt, Some(card))?;
            let text = match answer.as_str() {
                "" => preset_text(default),
                number => number
                    .parse::<usize>()
                    .ok()
                    .and_then(|number| PRESETS.get(number.wrapping_sub(1)))
                    .map_or(number, |preset| preset.1),
            };
            match NameTemplate::parse(text) {
                Ok(template) => return Ok(template),
                Err(error) => self.show_template_error(text, &error),
            }
        }
    }

    /// The number of the preset of the remembered template, or 1.
    fn remembered_preset(&self) -> usize {
        self.state
            .template
            .as_ref()
            .and_then(|template| {
                PRESETS
                    .iter()
                    .position(|preset| preset.1 == template.as_str())
            })
            .map_or(1, |index| index + 1)
    }

    /// Prints the template with a caret under the column of the error.
    fn show_template_error(&mut self, text: &str, error: &TemplateError) {
        self.say(&format!("  {text}"));
        let indent = " ".repeat(error.column.saturating_sub(1));
        self.say(&format!("  {indent}^ {}", error.kind));
    }

    /// Previews `template` with the first files of the card and asks whether to use it.
    fn preview(
        &mut self,
        card: &Device,
        scan: &SourceScan,
        template: &NameTemplate,
        job_code: Option<&JobCode>,
    ) -> Result<bool, Stop> {
        let preview_job_code = JobCode::parse(PREVIEW_JOB_CODE).ok();
        let destination = self.destination();
        let settings = import::ImportSettings {
            source: card.mount_point(),
            destination: &destination,
            template,
            job_code: job_code.or(preview_job_code.as_ref()),
            mode: naming::CompatibilityMode::On,
            policy: import::ConflictPolicy::AddSuffix,
            dry_run: true,
        };
        let ports = self.ports;
        let run = ImportRun::new(ports.file_system, ports.hasher, ports.clock);
        match run.plan_first(&settings, scan, PREVIEW_FILES) {
            Ok(plan) => self.show_preview(card, &plan),
            Err(error) => self.say(&format!("The template cannot be previewed: {error}")),
        }
        self.ask_yes("Use this template? [Y/n]: ", Some(card))
    }

    fn show_preview(&mut self, card: &Device, plan: &PlannedRun) {
        self.say(&format!(
            "Preview with the first {} files of {}:",
            plan.files().len(),
            card.name()
        ));
        for file in plan.files() {
            let target = match &file.planned {
                Ok(planned) => match &planned.target {
                    Ok(target) => target.clone(),
                    Err(error) => format!("cannot be named: {error}"),
                },
                Err(error) => format!("cannot be named: {error}"),
            };
            self.say(&format!("  {} -> {target}", file.found.path.display()));
        }
    }

    /// Asks for the job code of a card (CLI-002, "Job code").
    pub(super) fn ask_job_code(&mut self, card: &Device) -> Result<JobCode, Stop> {
        let recent = self.state.job_codes.job_codes().to_vec();
        if recent.is_empty() {
            return self.ask_new_job_code(card);
        }
        self.say("Job code:");
        for (index, job_code) in recent.iter().enumerate() {
            self.say(&format!("  {}  {}", index + 1, job_code.as_str()));
        }
        let choices = if recent.len() == 1 {
            "1".to_owned()
        } else {
            format!("1–{}", recent.len())
        };
        let prompt = format!("Choose {choices} or type a job code [1]: ");
        loop {
            let answer = self.prompt(&prompt, Some(card))?;
            let chosen = match answer.as_str() {
                "" => Some(0),
                number => number
                    .parse::<usize>()
                    .ok()
                    .and_then(|number| number.checked_sub(1)),
            };
            if let Some(job_code) = chosen.and_then(|index| recent.get(index)) {
                return Ok(job_code.clone());
            }
            if let Some(job_code) = self.checked_job_code(&answer) {
                return Ok(job_code);
            }
        }
    }

    /// Asks for a new job code when none is remembered; it has no default.
    fn ask_new_job_code(&mut self, card: &Device) -> Result<JobCode, Stop> {
        loop {
            let answer = self.prompt("Job code: ", Some(card))?;
            if answer.is_empty() {
                self.say("A job code is needed, because the template has {job}.");
            } else if let Some(job_code) = self.checked_job_code(&answer) {
                return Ok(job_code);
            }
        }
    }

    /// The job code `text`, or `None` after printing why it is invalid (IMPORT-006).
    fn checked_job_code(&mut self, text: &str) -> Option<JobCode> {
        match JobCode::parse(text) {
            Ok(job_code) => Some(job_code),
            Err(error) => {
                self.say(&format!("Invalid job code: {error}."));
                None
            }
        }
    }
}

/// The template text of the preset with `number`, counted from 1.
fn preset_text(number: usize) -> &'static str {
    PRESETS
        .get(number.saturating_sub(1))
        .map_or(PRESETS[0].1, |preset| preset.1)
}
