// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Format readiness: whether a finished import run saved every file of the source at
//! every required location, so the card can be formatted (IMPORT-008).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::backup::{BackupOutcome, BackupReport};
use crate::report::{FileOutcome, FileReport, ImportSummary};
use crate::scan::{LeftOut, LeftOutReason};

/// The note the format readiness of a camera run carries after its verdict
/// (IMPORT-014.10): the scan of a camera looks only below `DCIM`, so the readiness says
/// nothing about the files a phone keeps outside it.
pub const CAMERA_FORMAT_READINESS_NOTE: &str = "Only the files below DCIM were checked. \
     Files outside DCIM, such as a phone's other folders, are not backed up.";

/// Whether a finished import run verified every file of the source at the import
/// destination and at each backup destination, and which files keep the card from
/// being formatted (IMPORT-008). Derived from the [`ImportSummary`] alone: no file is
/// read and no destination is looked at again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatReadiness {
    /// The required locations: the import destination and each backup destination.
    /// With video destinations (IMPORT-013), each file is required at its own
    /// destinations; this is the most locations any file of the run requires. A run
    /// without files requires the locations a photo would (IMPORT-008.15).
    pub required_locations: usize,
    /// Whether the files of the run require different numbers of locations, as photos
    /// and videos do when their backup destinations differ in number (IMPORT-013).
    pub required_locations_differ: bool,
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
        /// How many locations the file itself requires (IMPORT-013).
        required_at: usize,
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
        let required_locations = summary
            .files
            .iter()
            .map(|file| file.required_locations)
            .max()
            .unwrap_or_else(|| photo_locations(summary));
        let required_locations_differ = summary
            .files
            .iter()
            .any(|file| file.required_locations != required_locations);
        let mut blockers = unsaved_files(&summary.files);
        blockers.extend(summary.left_out.iter().filter_map(blocking_left_out));
        FormatReadiness {
            required_locations,
            required_locations_differ,
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

/// The locations a photo requires: the import destination and each backup destination
/// that receives photos. A run without files requires these (IMPORT-008).
fn photo_locations(summary: &ImportSummary) -> usize {
    1 + summary
        .backups
        .iter()
        .filter(|counts| counts.receives_photos)
        .count()
}

/// The files and their carried-along sidecars not verified at every location they
/// require, with the number they are verified at; a duplicate counts as its earlier
/// file. A carried-along sidecar counts only where it was written and verified
/// (IMPORT-020.8).
fn unsaved_files(files: &[FileReport]) -> Vec<FormatBlocker> {
    let own_counts: HashMap<&Path, usize> = files
        .iter()
        .map(|file| (file.path.as_path(), verified_locations(file)))
        .collect();
    files
        .iter()
        .flat_map(|file| {
            let verified_at = match &file.outcome {
                FileOutcome::DuplicateOf { earlier } => {
                    own_counts.get(earlier.as_path()).copied().unwrap_or(0)
                }
                _ => verified_locations(file),
            };
            let sidecars = file
                .carried_sidecars
                .iter()
                .map(|sidecar| (&sidecar.path, sidecar.verified_at));
            std::iter::once((&file.path, verified_at))
                .chain(sidecars)
                .filter(|(_, verified_at)| *verified_at < file.required_locations)
                .map(|(path, verified_at)| FormatBlocker::NotSaved {
                    path: path.clone(),
                    verified_at,
                    required_at: file.required_locations,
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
        LeftOutReason::CannotBeRead(_)
        | LeftOutReason::SidecarNeedsGrouping
        | LeftOutReason::SidecarWithoutItsFile => true,
        LeftOutReason::UnsupportedType | LeftOutReason::Hidden | LeftOutReason::SymbolicLink => {
            false
        }
    }
}
