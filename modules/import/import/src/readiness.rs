// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Format readiness: whether a finished import run saved every file of the source at
//! every required location, so the card can be formatted (IMPORT-008).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::backup::{BackupOutcome, BackupReport};
use crate::report::{FileOutcome, FileReport, ImportSummary};
use crate::scan::{LeftOut, LeftOutReason};

/// Whether a finished import run verified every file of the source at the import
/// destination and at each backup destination, and which files keep the card from
/// being formatted (IMPORT-008). Derived from the [`ImportSummary`] alone: no file is
/// read and no destination is looked at again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatReadiness {
    /// The required locations: the import destination and each backup destination.
    pub required_locations: usize,
    /// The files and folders the scan left out for a reason that does not keep the
    /// card: an unsupported type, hidden, or a symbolic link. They are not saved; the
    /// other left-out entries are listed in `blockers` instead.
    pub left_out_not_saved: usize,
    /// Whether the progress sink or a failing import history stopped the run, so not
    /// every file was looked at.
    pub stopped: bool,
    /// Whether the run was a dry run, which verifies nothing.
    pub dry_run: bool,
    /// What keeps the card from being formatted: first the files found, then the files
    /// and folders left out, each in scan order.
    pub blockers: Vec<FormatBlocker>,
}

/// One file or folder that keeps the card from being formatted (IMPORT-008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatBlocker {
    /// A file the scan found that is not verified at every required location.
    NotSaved {
        /// The file, relative to the source.
        path: PathBuf,
        /// At how many of the required locations it is verified.
        verified_at: usize,
    },
    /// A file or folder left out for a reason that keeps the card.
    LeftOut {
        /// The file or folder, relative to the source.
        path: PathBuf,
        /// Why the scan left it out.
        reason: LeftOutReason,
    },
}

impl FormatReadiness {
    /// Evaluates the format readiness of the run that produced `summary`.
    #[must_use]
    pub fn of(summary: &ImportSummary) -> Self {
        let required_locations = 1 + summary.backups.len();
        let mut blockers = unsaved_files(&summary.files, required_locations);
        blockers.extend(summary.left_out.iter().filter_map(blocking_left_out));
        FormatReadiness {
            required_locations,
            left_out_not_saved: summary
                .left_out
                .iter()
                .filter(|left_out| !keeps_the_card(&left_out.reason))
                .count(),
            stopped: summary.stopped || summary.history_error.is_some(),
            dry_run: summary.dry_run,
            blockers,
        }
    }

    /// Whether the card can be formatted: the run finished, was not a dry run, and
    /// nothing keeps the card.
    #[must_use]
    pub fn can_be_formatted(&self) -> bool {
        !self.stopped && !self.dry_run && self.blockers.is_empty()
    }
}

/// The files not verified at every required location, with the number they are
/// verified at; a duplicate counts as its earlier file.
fn unsaved_files(files: &[FileReport], required_locations: usize) -> Vec<FormatBlocker> {
    let own_counts: HashMap<&Path, usize> = files
        .iter()
        .map(|file| (file.path.as_path(), verified_locations(file)))
        .collect();
    files
        .iter()
        .filter_map(|file| {
            let verified_at = match &file.outcome {
                FileOutcome::DuplicateOf { earlier } => {
                    own_counts.get(earlier.as_path()).copied().unwrap_or(0)
                }
                _ => verified_locations(file),
            };
            (verified_at < required_locations).then(|| FormatBlocker::NotSaved {
                path: file.path.clone(),
                verified_at,
            })
        })
        .collect()
}

/// At how many locations the file itself is verified: the import destination when it
/// was copied, already imported or imported before, and each backup destination that
/// has it or had it before (IMPORT-009, "Format readiness").
fn verified_locations(file: &FileReport) -> usize {
    let at_import_destination = matches!(
        file.outcome,
        FileOutcome::Copied { .. }
            | FileOutcome::AlreadyImported { .. }
            | FileOutcome::ImportedBefore { .. }
    );
    usize::from(at_import_destination) + file.backups.iter().filter(|b| is_backed_up(b)).count()
}

fn is_backed_up(backup: &BackupReport) -> bool {
    matches!(
        backup.outcome,
        BackupOutcome::BackedUp | BackupOutcome::AlreadyBackedUp | BackupOutcome::BackedUpBefore
    )
}

/// The left-out entry as a blocker, when its reason keeps the card.
fn blocking_left_out(left_out: &LeftOut) -> Option<FormatBlocker> {
    keeps_the_card(&left_out.reason).then(|| FormatBlocker::LeftOut {
        path: left_out.path.clone(),
        reason: left_out.reason.clone(),
    })
}

/// Whether a left-out entry keeps the card. Unsupported types, hidden files and
/// symbolic links do not: cameras write their own system files.
fn keeps_the_card(reason: &LeftOutReason) -> bool {
    match reason {
        LeftOutReason::CannotBeRead(_) | LeftOutReason::SidecarNeedsGrouping => true,
        LeftOutReason::UnsupportedType | LeftOutReason::Hidden | LeftOutReason::SymbolicLink => {
            false
        }
    }
}
