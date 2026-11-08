// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The lines of the guided import for an import that cannot be planned or started.

use devices::Device;
use history::HistoryError;
use import::RunError;

/// The line for a plan that cannot be made; a history that cannot be used has its own
/// line (CLI-008).
pub(super) fn plan_error_line(card: &Device, error: &RunError) -> String {
    match error {
        RunError::History(error) => history_line(error),
        error => format!("Could not plan {}: {error}", card.name()),
    }
}

/// The line for an import that could not start: the import history cannot be used
/// (CLI-008), a destination cannot be created (CLI-003), or another run error.
pub(super) fn start_error_line(card: &Device, error: &RunError) -> String {
    match error {
        RunError::History(error) => history_line(error),
        RunError::DestinationNotCreated(not_created) => format!(
            "Could not create {}: {}",
            not_created.destination.display(),
            not_created.error
        ),
        error => format!("Could not import {}: {error}", card.name()),
    }
}

/// The line for an import history that cannot be used (CLI-008).
pub(super) fn history_line(error: &HistoryError) -> String {
    format!(
        "The import history {} cannot be used: {}",
        error.file.display(),
        error.reason
    )
}
