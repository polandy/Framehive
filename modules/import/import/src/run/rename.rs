// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Naming the dated files of an import run, again when its sequence numbers change
//! (IMPORT-001, IMPORT-002).

use super::{ImportSettings, PlanInputs, PlannedRun};
use crate::plan::{AssetFile, ImportPlan};

/// The import plan of `naming`; each dated file is its own asset until grouping
/// (LIBRARY-001) exists.
pub(super) fn import_plan(naming: &PlanInputs, settings: &ImportSettings<'_>) -> ImportPlan {
    let assets: Vec<Vec<AssetFile>> = naming
        .assets
        .iter()
        .map(|file| vec![file.clone()])
        .collect();
    ImportPlan::new(
        &assets,
        settings.template,
        settings.mode,
        naming.import_time,
        settings.job_code,
        naming.first_sequence_number,
    )
}

/// Names the files of `plan` again from its inputs.
pub(super) fn rename(plan: &mut PlannedRun, settings: &ImportSettings<'_>) {
    let renamed = import_plan(&plan.naming, settings);
    // Exactly the dated files have a planned entry, in the order of the plan.
    let mut entries = renamed.files().iter();
    for file in &mut plan.files {
        if let Ok(planned) = &mut file.planned
            && let Some(entry) = entries.next()
        {
            *planned = entry.clone();
        }
    }
}
