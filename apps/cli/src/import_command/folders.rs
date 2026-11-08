// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The folders of `framehive import`: the source, the destination, the video
//! destination and the backup destinations, resolved and checked against each other
//! before anything is written (CLI-001, CLI-005, CLI-012).

use std::path::{Path, PathBuf};

use import::{
    BackupDestinationError, DestinationNotCreated, check_against_video_destination,
    check_backup_destinations, check_video_destination,
};

use super::ChosenSettings;
use crate::cameras::camera_choice;
use crate::paths::{Destination, PathError, check_outside, resolve_destination, resolve_source};
use crate::report::BackupName;

/// The folders of an import, resolved, each with the folder as given.
pub(super) struct Folders {
    /// The source folder, or the root `/` of a camera.
    pub(super) source: PathBuf,
    /// Whether the source is a camera (CLI-013), on a device of its own.
    pub(super) source_is_camera: bool,
    pub(super) destination: Destination,
    /// The destination as given on the command line or in the profile.
    pub(super) given_destination: PathBuf,
    /// Each backup destination with the folder as given.
    pub(super) backups: Vec<(Destination, PathBuf)>,
    /// The video destination with the folder as given, if any (CLI-012).
    pub(super) video_destination: Option<(Destination, PathBuf)>,
    /// Each video backup destination with the folder as given (CLI-012).
    pub(super) video_backups: Vec<(Destination, PathBuf)>,
}

impl Folders {
    /// The source the destinations are checked against: none for a camera, which no
    /// destination can lie inside.
    fn checked_source(&self) -> Option<&Path> {
        (!self.source_is_camera).then_some(self.source.as_path())
    }

    /// The resolved paths of the backup destinations, in their order.
    pub(super) fn backup_paths(&self) -> Vec<PathBuf> {
        paths_of(&self.backups)
    }

    /// The resolved paths of the video backup destinations, in their order.
    pub(super) fn video_backup_paths(&self) -> Vec<PathBuf> {
        paths_of(&self.video_backups)
    }

    /// The resolved path of the video destination, if any.
    pub(super) fn video_destination_path(&self) -> Option<&Path> {
        self.video_destination
            .as_ref()
            .map(|(video_destination, _)| video_destination.path.as_path())
    }

    /// A destination as the messages name it: as given (CLI-005, CLI-011, CLI-012).
    pub(super) fn shown<'p>(&'p self, path: &'p Path) -> &'p Path {
        if path == self.destination.path {
            return &self.given_destination;
        }
        self.video_destination
            .iter()
            .chain(&self.backups)
            .chain(&self.video_backups)
            .find(|(folder, _)| folder.path == path)
            .map_or(path, |(_, given)| given.as_path())
    }

    /// The message for a destination the job could not create when it started: for
    /// the destination the folder that failed, for the others also the kind and the
    /// folder as given (CLI-001, CLI-005, CLI-012).
    pub(super) fn not_created_message(&self, not_created: DestinationNotCreated) -> String {
        let DestinationNotCreated {
            destination,
            folder,
            error,
        } = not_created;
        let cannot = PathError::DestinationCannotBeCreated(folder, error);
        if destination == self.destination.path {
            return cannot.to_string();
        }
        // A folder that is both a backup and a video backup destination is named as a
        // backup destination, as the run creates the backup destinations first.
        let named = self
            .video_destination
            .iter()
            .map(|folder| ("video destination", folder))
            .chain(
                self.backups
                    .iter()
                    .map(|folder| ("backup destination", folder)),
            )
            .chain(
                self.video_backups
                    .iter()
                    .map(|folder| ("video backup destination", folder)),
            )
            .find(|(_, (folder, _))| folder.path == destination);
        match named {
            Some((kind, (_, given))) => unusable_message(kind, given, &cannot),
            None => cannot.to_string(),
        }
    }
}

/// The resolved paths of `folders`, in their order.
fn paths_of(folders: &[(Destination, PathBuf)]) -> Vec<PathBuf> {
    folders
        .iter()
        .map(|(folder, _)| folder.path.clone())
        .collect()
}

/// Resolves the folders of every source and checks them against each other before any
/// job starts (CLI-016): each source with the destinations, and the sources with each
/// other.
///
/// # Errors
///
/// Returns the message for the first source or folder that is not usable.
pub(super) fn resolve_sources(
    sources: &[PathBuf],
    chosen: &ChosenSettings,
) -> Result<Vec<Folders>, String> {
    let resolved = sources
        .iter()
        .map(|source| resolve_folders(source, chosen, camera_choice(source).is_some()))
        .collect::<Result<Vec<_>, _>>()?;
    let given_and_resolved: Vec<_> = sources.iter().zip(&resolved).collect();
    for (index, &later) in given_and_resolved.iter().enumerate() {
        for &earlier in &given_and_resolved[..index] {
            if let Some(message) = overlap(earlier, later) {
                return Err(message);
            }
        }
    }
    Ok(resolved)
}

/// A source as given, with its resolved folders.
type GivenSource<'a> = (&'a PathBuf, &'a Folders);

/// The message when the `later` source is the `earlier` one, or one lies inside the
/// other: compared by resolved path for folders, by the text after `camera:` for
/// cameras (CLI-016).
fn overlap(
    (earlier_given, earlier): GivenSource<'_>,
    (later_given, later): GivenSource<'_>,
) -> Option<String> {
    let given_twice = || {
        format!(
            "source {} is given twice; give it once",
            later_given.display()
        )
    };
    let inside = |inner: &Path, outer: &Path| {
        format!(
            "source {} lies inside the source {}; give it once",
            inner.display(),
            outer.display()
        )
    };
    match (earlier.source_is_camera, later.source_is_camera) {
        (true, true) => {
            (camera_choice(earlier_given) == camera_choice(later_given)).then(given_twice)
        }
        (false, false) if earlier.source == later.source => Some(given_twice()),
        (false, false) if later.source.starts_with(&earlier.source) => {
            Some(inside(later_given, earlier_given))
        }
        (false, false) if earlier.source.starts_with(&later.source) => {
            Some(inside(earlier_given, later_given))
        }
        _ => None,
    }
}

/// Resolves `given_source` and the destinations and checks them against each other
/// (CLI-001, CLI-005). A camera source (CLI-013) has the root `/` and is not resolved,
/// and no destination is checked against it.
///
/// # Errors
///
/// Returns the message for the first folder that cannot be resolved or is invalid.
fn resolve_folders(
    given_source: &Path,
    chosen: &ChosenSettings,
    source_is_camera: bool,
) -> Result<Folders, String> {
    let source = if source_is_camera {
        PathBuf::from("/")
    } else {
        resolve_source(given_source).map_err(|error| error.to_string())?
    };
    let destination =
        resolve_destination(&chosen.destination).map_err(|error| error.to_string())?;
    if !source_is_camera {
        check_outside(&source, &destination).map_err(|error| error.to_string())?;
    }
    let backups = resolve_backups(&chosen.backups, "backup destination")?;
    let video_destination = chosen
        .video_destination
        .as_ref()
        .map(|given| resolve_video_destination(given))
        .transpose()?;
    let video_backups = resolve_backups(&chosen.video_backups, "video backup destination")?;
    let folders = Folders {
        source,
        source_is_camera,
        destination,
        given_destination: chosen.destination.clone(),
        backups,
        video_destination,
        video_backups,
    };
    check_backup_destinations(
        folders.checked_source(),
        &folders.destination.path,
        &folders.backup_paths(),
    )
    .map_err(|error| invalid_backup_message(&error, &folders.backups, "backup destination"))?;
    check_video_folders(&folders)?;
    Ok(folders)
}

/// Checks the video destination and the video backup destinations against the other
/// folders before anything is written (IMPORT-013.8), naming each folder as given.
///
/// # Errors
///
/// Returns the message for the first invalid folder.
fn check_video_folders(folders: &Folders) -> Result<(), String> {
    let video_backups = folders.video_backup_paths();
    let video_backup_message = |error: BackupDestinationError| {
        invalid_backup_message(&error, &folders.video_backups, "video backup destination")
    };
    if let (Some((video_destination, given)), Some(source)) =
        (&folders.video_destination, folders.checked_source())
    {
        check_video_destination(source, &video_destination.path).map_err(|error| {
            format!(
                "invalid video destination {}: {}",
                given.display(),
                error.problem
            )
        })?;
    }
    if let Some(video_destination) = folders.video_destination_path() {
        check_against_video_destination(video_destination, &folders.backup_paths()).map_err(
            |error| invalid_backup_message(&error, &folders.backups, "backup destination"),
        )?;
    }
    check_backup_destinations(
        folders.checked_source(),
        &folders.destination.path,
        &video_backups,
    )
    .map_err(video_backup_message)?;
    if let Some(video_destination) = folders.video_destination_path() {
        check_against_video_destination(video_destination, &video_backups)
            .map_err(video_backup_message)?;
    }
    Ok(())
}

/// Resolves each backup destination as the destination is resolved, so that the
/// checks of IMPORT-007.11 and IMPORT-013.8 compare absolute paths without `..` or
/// symbolic links. `kind` names the folders in messages.
///
/// # Errors
///
/// Returns the message for the first backup destination that cannot be resolved.
fn resolve_backups(backups: &[PathBuf], kind: &str) -> Result<Vec<(Destination, PathBuf)>, String> {
    backups
        .iter()
        .map(|given| {
            resolve_destination(given)
                .map(|backup| (backup, given.clone()))
                .map_err(|error| unusable_message(kind, given, &error))
        })
        .collect()
}

/// Resolves the video destination as the destination is resolved (CLI-012).
///
/// # Errors
///
/// Returns the message when it cannot be resolved.
fn resolve_video_destination(given: &Path) -> Result<(Destination, PathBuf), String> {
    resolve_destination(given)
        .map(|video_destination| (video_destination, given.to_path_buf()))
        .map_err(|error| unusable_message("video destination", given, &error))
}

/// The backup destinations and video backup destinations as the report names them:
/// as given (CLI-005, CLI-012).
pub(super) fn backup_names(folders: &Folders) -> Vec<BackupName> {
    folders
        .backups
        .iter()
        .chain(&folders.video_backups)
        .map(|(backup, given)| BackupName {
            path: backup.path.clone(),
            shown: given.clone(),
        })
        .collect()
}

/// The message for a folder of the `kind` named, such as `backup destination`, that
/// cannot be resolved or created.
pub(super) fn unusable_message(kind: &str, given: &Path, error: &PathError) -> String {
    format!("the {kind} {} cannot be used: {error}", given.display())
}

/// The message for a folder of the `kind` named, such as `backup destination`, that
/// IMPORT-007.11 or IMPORT-013.8 rejects, naming the folder as given.
fn invalid_backup_message(
    error: &BackupDestinationError,
    backups: &[(Destination, PathBuf)],
    kind: &str,
) -> String {
    let given = backups
        .iter()
        .find(|(backup, _)| backup.path == error.path)
        .map_or(error.path.as_path(), |(_, given)| given.as_path());
    format!("invalid {kind} {}: {}", given.display(), error.problem)
}
