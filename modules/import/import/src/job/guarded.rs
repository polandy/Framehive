// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! How an import job reads its source through a stall guard (IMPORT-022, ADR 0049).

use std::path::Path;

use super::ImportPorts;
use super::sending::SendingSteps;
use crate::report::{ImportSummary, RunError};
use crate::reserved_names::ReservedNames;
use crate::run::{ImportSettings, PlannedRun};
use crate::source_guard::SourceGuard;

/// The source of a job, read through its guard, and its name if it is a source of its
/// own.
#[derive(Clone, Copy)]
pub(super) struct GuardedSource<'a> {
    pub(super) guard: &'a SourceGuard,
    pub(super) name: Option<&'a Path>,
}

/// Runs the import of `settings`, or starts and executes `plan`, reading the source
/// through its guard.
pub(super) fn run_through(
    ports: &ImportPorts,
    source: GuardedSource<'_>,
    settings: &ImportSettings<'_>,
    plan: Option<PlannedRun>,
    reserved: Option<&ReservedNames>,
    steps: &mut SendingSteps<'_>,
) -> Result<ImportSummary, RunError> {
    let run = ports
        .import_run()
        .with_reserved_names(&ports.reserved_names, reserved);
    let run = match source.name {
        Some(name) => run.with_source(source.guard).with_source_name(name),
        None => run.reading_source_through(source.guard),
    };
    match plan {
        Some(plan) => run
            .start(settings, plan)
            .map(|plan| run.execute_steps(settings, plan, steps)),
        None => run.run_steps(settings, steps),
    }
}
