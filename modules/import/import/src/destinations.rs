// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The destinations of each file of an import run, the checks of the video
//! destination and the video backup destinations before the run (IMPORT-013), and the
//! creation of the missing destinations when the run starts (IMPORT-012).

use std::path::{Path, PathBuf};
use std::{fmt, io};

use domain::AssetFileRole;
use files::FileSystem;

use crate::backup::{check_against_video_destination, check_backup_destinations};
use crate::report::RunError;
use crate::run::ImportSettings;

/// Why a video destination keeps the import run from starting (IMPORT-013.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VideoDestinationProblem {
    /// It lies inside the source, or is the source.
    InsideSource,
    /// It contains the source.
    ContainsSource,
}

impl fmt::Display for VideoDestinationProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            VideoDestinationProblem::InsideSource => "it lies inside the source",
            VideoDestinationProblem::ContainsSource => "it contains the source",
        })
    }
}

/// A video destination that keeps the import run from starting; nothing is written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid video destination {}: {problem}", path.display())]
pub struct VideoDestinationError {
    /// The video destination, as given.
    pub path: PathBuf,
    /// What is wrong with it.
    pub problem: VideoDestinationProblem,
}

/// Checks the video destination against the source (IMPORT-013, "Before the run").
/// The paths are compared as given: both are absolute and without `..`.
///
/// # Errors
///
/// Returns a [`VideoDestinationError`] if the video destination lies inside the
/// source or contains it.
pub fn check_video_destination(
    source: &Path,
    video_destination: &Path,
) -> Result<(), VideoDestinationError> {
    let problem = if video_destination.starts_with(source) {
        VideoDestinationProblem::InsideSource
    } else if source.starts_with(video_destination) {
        VideoDestinationProblem::ContainsSource
    } else {
        return Ok(());
    };
    Err(VideoDestinationError {
        path: video_destination.to_path_buf(),
        problem,
    })
}

impl<'a> ImportSettings<'a> {
    /// The destination a file with `role` is copied to: the video destination for a
    /// video when one is given, otherwise the import destination (IMPORT-013).
    #[must_use]
    pub fn destination_of(&self, role: AssetFileRole) -> &'a Path {
        match (role, self.video_destination) {
            (AssetFileRole::Video, Some(video_destination)) => video_destination,
            _ => self.destination,
        }
    }

    /// The backup destinations a file with `role` is backed up to: the video backup
    /// destinations for a video when any are given, otherwise the backup destinations
    /// (IMPORT-013).
    #[must_use]
    pub fn backup_destinations_of(&self, role: AssetFileRole) -> &'a [PathBuf] {
        if role == AssetFileRole::Video && !self.video_backup_destinations.is_empty() {
            self.video_backup_destinations
        } else {
            self.backup_destinations
        }
    }

    /// The video destination when it is a folder of its own, not the import
    /// destination.
    pub(crate) fn separate_video_destination(&self) -> Option<&'a Path> {
        self.video_destination
            .filter(|video_destination| *video_destination != self.destination)
    }

    /// Every backup destination of the run, once each: the backup destinations, then
    /// the video backup destinations that are not also backup destinations.
    pub(crate) fn all_backup_destinations(&self) -> Vec<&'a Path> {
        let mut all: Vec<&'a Path> = self
            .backup_destinations
            .iter()
            .map(PathBuf::as_path)
            .collect();
        for video_backup in self.video_backup_destinations {
            if !all.contains(&video_backup.as_path()) {
                all.push(video_backup);
            }
        }
        all
    }

    /// Every destination of the run, once each: the import destination, the video
    /// destination, then [`ImportSettings::all_backup_destinations`].
    pub(crate) fn all_destinations(&self) -> Vec<&'a Path> {
        let mut all = vec![self.destination];
        all.extend(self.separate_video_destination());
        all.extend(self.all_backup_destinations());
        all
    }
}

/// Checks the backup destinations, the video destination and the video backup
/// destinations of `settings` before anything is written (IMPORT-007.11,
/// IMPORT-013.8). A `source` of `None` lies on a device of its own, such as a camera
/// (ADR 0034), so no destination is checked against it.
pub(crate) fn check_destinations(
    settings: &ImportSettings<'_>,
    source: Option<&Path>,
) -> Result<(), RunError> {
    let destination = settings.destination;
    check_backup_destinations(source, destination, settings.backup_destinations)?;
    if let Some(video_destination) = settings.video_destination {
        if let Some(source) = source {
            check_video_destination(source, video_destination)?;
        }
        check_against_video_destination(video_destination, settings.backup_destinations)?;
    }
    let video_backups = settings.video_backup_destinations;
    check_backup_destinations(source, destination, video_backups)
        .map_err(RunError::VideoBackupDestination)?;
    if let Some(video_destination) = settings.video_destination {
        check_against_video_destination(video_destination, video_backups)
            .map_err(RunError::VideoBackupDestination)?;
    }
    Ok(())
}

/// A destination of the run, or one of its parents, cannot be created when the run
/// starts (IMPORT-012). Nothing is reserved or copied; the folders created before
/// it stay.
#[derive(Debug, thiserror::Error)]
#[error(
    "the destination {} cannot be created: the folder {} cannot be created: {error}",
    .destination.display(),
    .folder.display()
)]
pub struct DestinationNotCreated {
    /// The destination as the settings give it.
    pub destination: PathBuf,
    /// The folder that could not be created: the destination or one of its parents.
    pub folder: PathBuf,
    /// Why it could not be created.
    pub error: io::Error,
}

/// Creates every missing destination of `settings` with its parents, outermost first
/// (IMPORT-012). A folder that exists, also one another program created meanwhile, is
/// left as it is.
pub(crate) fn create_destinations(
    file_system: &dyn FileSystem,
    settings: &ImportSettings<'_>,
) -> Result<(), DestinationNotCreated> {
    for destination in settings.all_destinations() {
        create_with_parents(file_system, destination).map_err(|(folder, error)| {
            DestinationNotCreated {
                destination: destination.to_path_buf(),
                folder,
                error,
            }
        })?;
    }
    Ok(())
}

/// Creates `folder` and its missing parents, and syncs each new folder into its
/// parent, so that it survives a crash as the folders of a copy do (IMPORT-004).
fn create_with_parents(
    file_system: &dyn FileSystem,
    folder: &Path,
) -> Result<(), (PathBuf, io::Error)> {
    let mut missing = Vec::new();
    for ancestor in folder.ancestors() {
        if ancestor.as_os_str().is_empty() || exists(file_system, ancestor)? {
            break;
        }
        missing.push(ancestor);
    }
    for path in missing.into_iter().rev() {
        match file_system.create_folder(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err((path.to_path_buf(), error)),
        }
        let parent = path.parent().unwrap_or(Path::new(""));
        file_system
            .sync_folder(parent)
            .map_err(|error| (path.to_path_buf(), error))?;
    }
    Ok(())
}

/// Whether anything is at `path`.
fn exists(file_system: &dyn FileSystem, path: &Path) -> Result<bool, (PathBuf, io::Error)> {
    let entry = file_system
        .entry(path)
        .map_err(|error| (path.to_path_buf(), error))?;
    Ok(entry.is_some())
}
