// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Turns the source and destination of the command line into the absolute paths
//! without `..` that an import run needs, and creates a missing destination.

use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};

use files::FileSystem;

/// The import destination, resolved, with the folders of it that do not exist yet.
#[derive(Debug)]
pub(crate) struct Destination {
    /// The absolute path of the destination.
    pub(crate) path: PathBuf,
    /// The folders of `path` that are missing, outermost first.
    missing: Vec<PathBuf>,
}

/// Why the source or destination cannot be used; the import does not start.
#[derive(Debug)]
pub(crate) enum PathError {
    /// Nothing exists at the source path.
    SourceMissing(PathBuf),
    /// The source path cannot be resolved.
    SourceCannotBeRead(PathBuf, io::Error),
    /// The working folder, needed for a relative path, cannot be read.
    WorkingFolder(io::Error),
    /// The missing part of the destination steps up with `..`, which cannot be
    /// resolved without the folders existing.
    ParentStepInDestination(PathBuf),
    /// A part of the destination cannot be resolved.
    DestinationCannotBeRead(PathBuf, io::Error),
    /// The destination exists, but is not a folder.
    DestinationNotAFolder(PathBuf),
    /// The destination lies inside the source, or is the source.
    DestinationInsideSource {
        /// The source folder.
        source: PathBuf,
        /// The import destination.
        destination: PathBuf,
    },
    /// A folder of the destination cannot be created.
    DestinationCannotBeCreated(PathBuf, io::Error),
}

impl fmt::Display for PathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::SourceMissing(path) => {
                write!(formatter, "the source {} does not exist", path.display())
            }
            PathError::SourceCannotBeRead(path, error) => {
                write!(
                    formatter,
                    "the source {} cannot be read: {error}",
                    path.display()
                )
            }
            PathError::WorkingFolder(error) => {
                write!(formatter, "the working folder cannot be read: {error}")
            }
            PathError::ParentStepInDestination(path) => write!(
                formatter,
                "the destination {} steps up with .. below a folder that does not exist",
                path.display()
            ),
            PathError::DestinationCannotBeRead(path, error) => {
                write!(
                    formatter,
                    "the destination {} cannot be read: {error}",
                    path.display()
                )
            }
            PathError::DestinationNotAFolder(path) => {
                write!(
                    formatter,
                    "the destination {} is not a folder",
                    path.display()
                )
            }
            PathError::DestinationInsideSource {
                source,
                destination,
            } => write!(
                formatter,
                "the destination {} lies inside the source {}; choose a destination outside it",
                destination.display(),
                source.display()
            ),
            PathError::DestinationCannotBeCreated(path, error) => {
                write!(
                    formatter,
                    "the destination {} cannot be created: {error}",
                    path.display()
                )
            }
        }
    }
}

/// Resolves the source to an absolute path without `..` or symbolic links.
///
/// # Errors
///
/// Returns a [`PathError`] if the source does not exist or cannot be resolved.
pub(crate) fn resolve_source(source: &Path) -> Result<PathBuf, PathError> {
    std::fs::canonicalize(source).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            PathError::SourceMissing(source.to_path_buf())
        } else {
            PathError::SourceCannotBeRead(source.to_path_buf(), error)
        }
    })
}

/// Resolves the destination: its longest existing part without symbolic links, with
/// the missing folders appended.
///
/// # Errors
///
/// Returns a [`PathError`] if the missing part contains `..`, a part cannot be
/// resolved, or the destination exists but is not a folder.
pub(crate) fn resolve_destination(destination: &Path) -> Result<Destination, PathError> {
    let absolute = absolute(destination)?;
    let components: Vec<Component<'_>> = absolute.components().collect();
    // The root always exists, so the loop ends with an existing part at the latest.
    for split in (1..=components.len()).rev() {
        let prefix: PathBuf = components[..split].iter().collect();
        match std::fs::canonicalize(&prefix) {
            Ok(existing) => return with_missing(destination, existing, &components[split..]),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(PathError::DestinationCannotBeRead(prefix, error)),
        }
    }
    Err(PathError::DestinationCannotBeRead(
        absolute.clone(),
        io::Error::from(io::ErrorKind::NotFound),
    ))
}

fn absolute(path: &Path) -> Result<PathBuf, PathError> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let working_folder = std::env::current_dir().map_err(PathError::WorkingFolder)?;
    Ok(working_folder.join(path))
}

fn with_missing(
    given: &Path,
    existing: PathBuf,
    rest: &[Component<'_>],
) -> Result<Destination, PathError> {
    let mut path = existing.clone();
    let mut missing = Vec::with_capacity(rest.len());
    for component in rest {
        match component {
            Component::Normal(name) => {
                path.push(name);
                missing.push(path.clone());
            }
            Component::CurDir => {}
            _ => return Err(PathError::ParentStepInDestination(given.to_path_buf())),
        }
    }
    if missing.is_empty() && !existing.is_dir() {
        return Err(PathError::DestinationNotAFolder(existing));
    }
    Ok(Destination { path, missing })
}

/// Rejects a destination inside the source, or equal to it (CLI-001, exit code 2).
///
/// # Errors
///
/// Returns [`PathError::DestinationInsideSource`] with both paths.
pub(crate) fn check_outside(source: &Path, destination: &Destination) -> Result<(), PathError> {
    if destination.path.starts_with(source) {
        return Err(PathError::DestinationInsideSource {
            source: source.to_path_buf(),
            destination: destination.path.clone(),
        });
    }
    Ok(())
}

/// Creates the missing folders of the destination, outermost first.
///
/// # Errors
///
/// Returns [`PathError::DestinationCannotBeCreated`] for the first folder that cannot
/// be created; the folders created before it stay.
pub(crate) fn create_destination(
    file_system: &dyn FileSystem,
    destination: &Destination,
) -> Result<(), PathError> {
    for folder in &destination.missing {
        match file_system.create_folder(folder) {
            Ok(()) => {}
            // Created by someone else in the meantime; the import checks it is a folder.
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(PathError::DestinationCannotBeCreated(folder.clone(), error)),
        }
    }
    Ok(())
}
