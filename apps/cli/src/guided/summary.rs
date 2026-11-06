// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The plan summary of the guided import: what it shows, what keeps Enter from
//! starting the import, and the choices (CLI-003, CLI-006).

use std::path::Path;

use devices::Device;
use domain::LocalDateTime;
use import::{
    BackupDestinationProblem, ImportRun, ImportSettings, PlannedRun, SpaceShortage,
    check_backup_destinations,
};

use super::dialog::{Dialog, Stop};
use super::format::{size_text, space_text};

/// The line under a summary whose destination lies inside the card.
const ON_CARD: &str = "The destination is on the card; type d to choose another.";

/// What the person chose at the plan summary.
pub(super) enum Action {
    Import,
    List,
    JobCode,
    Template,
    Destination,
    Backup,
}

impl Dialog<'_> {
    /// Prints the plan summary (CLI-003, "Plan summary").
    pub(super) fn show_summary(
        &mut self,
        card: &Device,
        plan: &PlannedRun,
        settings: &ImportSettings<'_>,
    ) {
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
        if settings.backup_destinations.is_empty() {
            self.say("  backup    none");
        }
        for backup in settings.backup_destinations {
            self.say(&format!("  backup    {}", backup.display()));
        }
        self.say(&format!("  named     {}", settings.template.as_str()));
        if settings.template.uses_job_code()
            && let Some(job_code) = settings.job_code
        {
            self.say(&format!("  job code  {}", job_code.as_str()));
        }
    }

    /// The lines that keep the import of `plan` from starting, and a warning for each
    /// destination whose free space is unknown (CLI-003, CLI-006, CLI-011). A `camera`
    /// has no destination on the card.
    pub(super) fn blockers(
        &mut self,
        run: &ImportRun<'_>,
        settings: &ImportSettings<'_>,
        plan: &PlannedRun,
        camera: bool,
    ) -> Vec<String> {
        // No destination can lie on a camera (CLI-014).
        let card = (!camera).then_some(settings.source);
        let mut lines = blockers(settings, card);
        let space = run.check_free_space(settings, plan);
        lines.extend(space_blockers(&space.shortages));
        for unknown in &space.unknown {
            self.warn(&unknown.to_string());
        }
        lines
    }

    /// Asks what to do at the plan summary.
    pub(super) fn summary_action(
        &mut self,
        card: &Device,
        uses_job_code: bool,
        blockers: &[String],
    ) -> Result<Action, Stop> {
        let (prompt, hint) = if uses_job_code {
            (
                "[Enter] import  [l] list  [j] job code  [t] template  [d] destination  [b] backup  [q] quit: ",
                "Type l, j, t, d, b or q, or press Enter to import.",
            )
        } else {
            (
                "[Enter] import  [l] list  [t] template  [d] destination  [b] backup  [q] quit: ",
                "Type l, t, d, b or q, or press Enter to import.",
            )
        };
        loop {
            match self.prompt(prompt, Some(card))?.as_str() {
                "" if !blockers.is_empty() => {
                    for blocker in blockers {
                        self.say(blocker);
                    }
                }
                "" => return Ok(Action::Import),
                "l" => return Ok(Action::List),
                "j" if uses_job_code => return Ok(Action::JobCode),
                "t" => return Ok(Action::Template),
                "d" => return Ok(Action::Destination),
                "b" => return Ok(Action::Backup),
                _ => self.say(hint),
            }
        }
    }
}

/// The lines that keep the import from starting: a destination on the card, and a
/// backup destination that is invalid (CLI-003, CLI-006). `card` is the source the
/// destinations are checked against, none for a camera. The free space is checked
/// apart, in [`space_blockers`].
fn blockers(settings: &ImportSettings<'_>, card: Option<&Path>) -> Vec<String> {
    let mut blockers = Vec::new();
    if card.is_some_and(|card| settings.destination.starts_with(card)) {
        blockers.push(ON_CARD.to_owned());
    }
    if let Err(error) =
        check_backup_destinations(card, settings.destination, settings.backup_destinations)
    {
        let reason = match error.problem {
            BackupDestinationProblem::InsideSource => "it is on the card".to_owned(),
            problem => problem.to_string(),
        };
        blockers.push(format!(
            "The backup folder {} cannot be used: {reason}; type b to change it.",
            error.path.display()
        ));
    }
    blockers
}

/// The line under a summary for each file system that cannot hold what the import
/// writes to it (CLI-011).
fn space_blockers(shortages: &[SpaceShortage]) -> Vec<String> {
    shortages
        .iter()
        .map(|shortage| {
            let folders = shortage
                .destinations
                .iter()
                .map(|folder| folder.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "Not enough space at {folders}: {}; type d or b to change a folder.",
                space_text(shortage)
            )
        })
        .collect()
}

/// `2026-10-17 to 2026-10-19`, or one date, for the planned files with a date.
fn captured_text(plan: &PlannedRun) -> Option<String> {
    let dates = plan
        .files()
        .iter()
        .filter_map(|file| file.planned.as_ref().ok()?.named())
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
