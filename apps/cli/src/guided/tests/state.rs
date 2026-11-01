// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The state file of the guided import (CLI-003, ADR 0025).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use domain::Timestamp;
use files::testing::MemoryFileSystem;
use import::RecentJobCodes;
use naming::NameTemplate;

use crate::guided::state::{
    RememberedState, format_state, load_state, parse_state, save_state, state_file,
};

const PRESET_2: &str = "{date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}";

fn example_state() -> RememberedState {
    RememberedState {
        destination: Some(PathBuf::from("/home/ada/Pictures")),
        template: Some(NameTemplate::parse(PRESET_2).unwrap()),
        job_codes: RecentJobCodes::from_stored(["Smith wedding", "Harbour"]),
    }
}

/// spec: CLI-003.11
#[test]
fn the_state_file_has_the_lines_of_adr_0025_and_reads_back() {
    let file_system = MemoryFileSystem::new();
    let path = state_file(Some(OsStr::new("/home/ada/.state")), Path::new("/home/ada"));

    save_state(&file_system, &path, &example_state()).unwrap();

    assert_eq!(
        path,
        Path::new("/home/ada/.state/framehive/guided-import.txt")
    );
    let text = String::from_utf8(file_system.content(&path).unwrap()).unwrap();
    assert_eq!(
        text,
        "# Framehive remembers the answers of the guided import here.\n\
         format_version=1\n\
         destination=/home/ada/Pictures\n\
         template={date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}\n\
         job_code=Smith wedding\n\
         job_code=Harbour\n"
    );
    let loaded = load_state(&file_system, &path);
    assert_eq!(loaded.state, example_state());
    assert!(loaded.warnings.is_empty());
}

/// spec: CLI-003.11
#[test]
fn writing_again_replaces_the_file_and_leaves_no_part_file() {
    let file_system = MemoryFileSystem::new();
    let path = Path::new("/state/framehive/guided-import.txt");
    save_state(&file_system, path, &RememberedState::default()).unwrap();

    save_state(&file_system, path, &example_state()).unwrap();

    assert_eq!(load_state(&file_system, path).state, example_state());
    assert_eq!(
        file_system.content(Path::new("/state/framehive/guided-import.txt.part")),
        None
    );
}

/// spec: CLI-003.12
#[test]
fn without_an_absolute_xdg_state_home_the_file_is_below_the_home_folder() {
    let expected = Path::new("/home/ada/.local/state/framehive/guided-import.txt");

    assert_eq!(state_file(None, Path::new("/home/ada")), expected);
    assert_eq!(
        state_file(Some(OsStr::new("relative/path")), Path::new("/home/ada")),
        expected
    );
}

/// spec: CLI-003.10
#[test]
fn another_format_version_ignores_the_file_with_a_warning() {
    let file_system = MemoryFileSystem::new();
    let path = Path::new("/state/guided-import.txt");
    let text = "format_version=7\ndestination=/home/ada/Pictures\n";
    file_system.add_file(path, text.as_bytes(), Timestamp::from_unix_seconds(0));

    let loaded = load_state(&file_system, path);

    assert_eq!(loaded.state, RememberedState::default());
    assert_eq!(loaded.warnings.len(), 1);
    assert!(
        loaded.warnings[0].contains("format_version=7"),
        "{:?}",
        loaded.warnings
    );
}

#[test]
fn a_missing_file_is_a_first_run_without_warning() {
    let loaded = load_state(&MemoryFileSystem::new(), Path::new("/state/x.txt"));

    assert_eq!(loaded.state, RememberedState::default());
    assert!(loaded.warnings.is_empty());
}

#[test]
fn a_line_that_is_not_understood_ignores_the_file() {
    for text in [
        "format_version=1\ncolour=blue\n",
        "format_version=1\nno equals sign\n",
        "format_version=1\ntemplate={name}.{ext}\ntemplate={name}.{ext}\n",
        "# only a comment\n",
    ] {
        assert!(parse_state(text).is_err(), "{text:?}");
    }
}

#[test]
fn invalid_values_are_dropped_with_a_warning_and_the_rest_is_kept() {
    let text = "\n# comment\nformat_version=1\ndestination=relative\n\
                template={camera}\njob_code=a/b\njob_code=Harbour\n";

    let loaded = parse_state(text).unwrap();

    assert_eq!(loaded.state.destination, None);
    assert_eq!(loaded.state.template, None);
    assert_eq!(
        loaded.state.job_codes,
        RecentJobCodes::from_stored(["Harbour"])
    );
    assert_eq!(loaded.warnings.len(), 3, "{:?}", loaded.warnings);
}

#[test]
fn a_destination_with_a_line_break_is_not_written() {
    let state = RememberedState {
        destination: Some(PathBuf::from("/home/ada/a\nb")),
        ..RememberedState::default()
    };

    assert!(!format_state(&state).contains("destination="));
}
