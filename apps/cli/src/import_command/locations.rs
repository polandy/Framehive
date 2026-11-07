// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The sources and the destination of `framehive import`, as `cp` names them: the last
//! positional argument is the destination, unless `--destination` names it (CLI-016).
//! This depends on the arguments alone, never on the profile or the file system.

use std::path::{Path, PathBuf};

use super::run::NotStarted;
use crate::arguments::ImportArguments;
use crate::cameras::camera_choice;

/// The start of the message for a command without a source.
const NO_SOURCE: &str = "no source: give the folders or cameras to import from";

/// The folders of the command line, as given.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Locations {
    /// The sources, in the order given (CLI-015).
    pub(super) sources: Vec<PathBuf>,
    /// The folder to import into.
    pub(super) destination: PathBuf,
}

/// Splits the positional arguments into the sources and the destination (CLI-016).
///
/// # Errors
///
/// Returns the message, and the hint when there is one, for a command without a
/// source, without a destination, or with a camera where the destination belongs.
pub(super) fn locations(arguments: &ImportArguments) -> Result<Locations, NotStarted> {
    let positionals = &arguments.positionals;
    if let Some(destination) = &arguments.destination {
        if positionals.is_empty() {
            return Err(NotStarted::with_hint(
                NO_SOURCE.to_owned(),
                format!(
                    "framehive import --destination {} /media/card",
                    destination.display()
                ),
            ));
        }
        return Ok(Locations {
            sources: positionals.clone(),
            destination: destination.clone(),
        });
    }
    match positionals.split_last() {
        None => Err(NotStarted::with_hint(
            format!("{NO_SOURCE}, then the folder to import into"),
            "framehive import /media/card ~/Pictures".to_owned(),
        )),
        Some((only, [])) => Err(no_destination(only)),
        Some((last, _)) if camera_choice(last).is_some() => Err(NotStarted::from(format!(
            "{} cannot be the destination; give the folder to import into last, or with --destination",
            last.display()
        ))),
        Some((last, sources)) => Ok(Locations {
            sources: sources.to_vec(),
            destination: last.clone(),
        }),
    }
}

/// The error for a single positional argument, which is the source: the destination
/// is missing.
fn no_destination(source: &Path) -> NotStarted {
    let source = source.display();
    NotStarted::with_hint(
        format!(
            "no destination: give the folder to import into after {source}, or with --destination"
        ),
        format!("framehive import {source} ~/Pictures"),
    )
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;
    use crate::arguments::{Invocation, parse};

    fn located(arguments: &[&str]) -> Result<Locations, NotStarted> {
        let line = ["framehive", "import"]
            .iter()
            .chain(arguments)
            .map(OsString::from);
        let Ok(Invocation::Import(arguments)) = parse(line) else {
            panic!("not an import");
        };
        locations(&arguments)
    }

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn the_last_folder_is_the_destination() {
        let found = located(&["cardA", "camera:", "library"]).unwrap();
        assert_eq!(found.sources, paths(&["cardA", "camera:"]));
        assert_eq!(found.destination, PathBuf::from("library"));
    }

    #[test]
    fn with_destination_every_folder_is_a_source() {
        let found = located(&["cardA", "cardB", "--destination", "library"]).unwrap();
        assert_eq!(found.sources, paths(&["cardA", "cardB"]));
        assert_eq!(found.destination, PathBuf::from("library"));
        let found = located(&["--destination", "library", "camera:"]).unwrap();
        assert_eq!(found.sources, paths(&["camera:"]));
    }

    #[test]
    fn a_folder_named_like_a_camera_can_be_the_destination() {
        let found = located(&["camera:", "./camera:library"]).unwrap();
        assert_eq!(found.destination, PathBuf::from("./camera:library"));
    }

    #[test]
    fn a_single_camera_lacks_a_destination() {
        let Err(not_started) = located(&["camera:"]) else {
            panic!("a single camera has no destination");
        };
        assert_eq!(
            not_started.messages,
            ["no destination: give the folder to import into after camera:, or with --destination"]
        );
        assert_eq!(
            not_started.hint.as_deref(),
            Some("framehive import camera: ~/Pictures")
        );
    }
}
