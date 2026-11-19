// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The free-space check of `framehive import` before a job starts, and its messages
//! with the folders as given (CLI-011).

use std::io::Write;
use std::path::PathBuf;

use import::{
    FreeSpaceUnknown, ImportRun, ImportSettings, PlannedRun, SharedFileSystem, SpaceShortage,
};

use super::folders::Folders;
use super::run::NotStarted;
use crate::guided::space_text;
use crate::write_warning;

/// Checks that each file system can hold what the run writes to it, before any folder
/// is created, and warns about each destination whose free space is unknown and each
/// backup on the file system of an earlier destination (CLI-011);
/// returns those destinations.
///
/// # Errors
///
/// Returns one message per file system that is short.
pub(super) fn check_free_space(
    run: &ImportRun<'_>,
    settings: &ImportSettings<'_>,
    folders: &Folders,
    plan: &PlannedRun,
    errors: &mut dyn Write,
) -> Result<Vec<PathBuf>, NotStarted> {
    let check = run.check_free_space(settings, plan);
    for unknown in &check.unknown {
        write_warning(errors, &unknown_free_space_message(unknown, folders));
    }
    for shared in &check.shared {
        write_warning(errors, &shared_file_system_message(shared, folders));
    }
    if check.shortages.is_empty() {
        return Ok(check
            .unknown
            .into_iter()
            .map(|unknown| unknown.destination)
            .collect());
    }
    Err(NotStarted {
        messages: check
            .shortages
            .iter()
            .map(|shortage| shortage_message(shortage, folders))
            .collect(),
        hint: None,
    })
}

/// `cannot tell the free space at <folder as given>: <reason>` (CLI-011).
pub(super) fn unknown_free_space_message(unknown: &FreeSpaceUnknown, folders: &Folders) -> String {
    format!(
        "cannot tell the free space at {}: {}",
        folders.shown(&unknown.destination).display(),
        unknown.error
    )
}

/// `backup <folder> is on the same file system as <folder>; ...`, with the folders as
/// given (CLI-011).
fn shared_file_system_message(shared: &SharedFileSystem, folders: &Folders) -> String {
    SharedFileSystem {
        backup_destination: folders.shown(&shared.backup_destination).to_path_buf(),
        shared_with: folders.shown(&shared.shared_with).to_path_buf(),
    }
    .to_string()
}

/// `not enough space at <folders as given>: needs 12.8 GB, 3.1 GB free` (CLI-011).
pub(super) fn shortage_message(shortage: &SpaceShortage, folders: &Folders) -> String {
    let shown = shortage
        .destinations
        .iter()
        .map(|destination| folders.shown(destination).display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("not enough space at {shown}: {}", space_text(shortage))
}
