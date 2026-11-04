// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The plan summary of the guided import: what it shows, what keeps Enter from
//! starting the import, and the choices (CLI-003, CLI-006).

use devices::Device;
use domain::LocalDateTime;
use import::{
    BackupDestinationProblem, ImportSettings, PlanNotice, PlannedRun, check_backup_destinations,
};

use super::dialog::{Dialog, Stop};
use super::format::size_text;

/// The line under a summary whose destination lies inside the card.
const ON_CARD: &str = "The destination is on the card; type d to choose another.";
/// The plan notice of a persisted sequence counter (CLI-003, IMPORT-002).
const NOT_RECOGNIZED: &str = "Files already imported are not recognized, because the template has a persisted sequence counter.";

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
        if plan
            .notices()
            .contains(&PlanNotice::AlreadyImportedNotRecognized)
        {
            self.say(NOT_RECOGNIZED);
        }
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
/// backup destination that is invalid (CLI-003, CLI-006).
pub(super) fn blockers(settings: &ImportSettings<'_>) -> Vec<String> {
    let mut blockers = Vec::new();
    if settings.destination.starts_with(settings.source) {
        blockers.push(ON_CARD.to_owned());
    }
    if let Err(error) = check_backup_destinations(
        settings.source,
        settings.destination,
        settings.backup_destinations,
    ) {
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
