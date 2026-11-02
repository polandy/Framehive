// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The home folder, the current folder and the state file the dialog starts from.

use std::io;
use std::path::{Path, PathBuf};

use crate::guided::environment_from;

/// The current folder of a process that can read it.
const WORK: &str = "/tmp/work";

/// The error message for the given home folder and current folder.
fn error_for(home: Option<&str>, current_folder: io::Result<PathBuf>) -> String {
    environment_from(home.map(PathBuf::from), current_folder, None).unwrap_err()
}

/// spec: CLI-002.15
#[test]
fn a_missing_home_folder_is_unknown() {
    let message = error_for(None, Ok(PathBuf::from(WORK)));

    assert_eq!(message, "the home folder is unknown; set HOME");
}

/// spec: CLI-002.15
#[test]
fn a_relative_home_folder_is_unknown() {
    for home in ["", "ada"] {
        let message = error_for(Some(home), Ok(PathBuf::from(WORK)));

        assert_eq!(
            message, "the home folder is unknown; set HOME",
            "home {home:?}"
        );
    }
}

/// spec: CLI-002.15
#[test]
fn an_unreadable_current_folder_is_unknown() {
    let deleted = io::Error::new(io::ErrorKind::NotFound, "deleted");

    let message = error_for(Some("/home/ada"), Err(deleted));

    assert_eq!(message, "the current folder cannot be read: deleted");
}

#[test]
fn known_folders_give_the_environment() {
    let environment = environment_from(
        Some(PathBuf::from("/home/ada")),
        Ok(PathBuf::from(WORK)),
        None,
    )
    .unwrap();

    assert_eq!(environment.home, Path::new("/home/ada"));
    assert_eq!(environment.current_folder, Path::new("/tmp/work"));
    assert_eq!(
        environment.state_file,
        Path::new("/home/ada/.local/state/framehive/guided-import.txt")
    );
}
