// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import --profile` (CLI-010) against the real binary, the real file system
//! and a temporary folder that holds the home folder. The child process runs with
//! `TZ=UTC`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use support::{DEFAULT_TARGET, Folders, stderr, stdout};

const DATED_TARGET: &str = "20261017-DSCF0001.RAF";

fn text(path: &Path) -> String {
    path.display().to_string()
}

/// An empty folder below the root.
fn empty_folder(folders: &Folders, name: &str) -> PathBuf {
    let folder = folders.root.path().join(name);
    fs::create_dir_all(&folder).unwrap();
    folder
}

/// The profile `holiday` of CLI-010.1: a template and one backup destination, an
/// empty folder, and the empty folder to give as the destination.
struct Holiday {
    destination: PathBuf,
    backup: PathBuf,
}

fn holiday(folders: &Folders) -> Holiday {
    let holiday = Holiday {
        destination: empty_folder(folders, "holiday-library"),
        backup: empty_folder(folders, "holiday-backup"),
    };
    folders.write_profile(
        "holiday",
        format!(
            "schema_version: 1\nnaming:\n  template: \"{{date:%Y%m%d}}-{{name}}.{{ext}}\"\nbackup:\n  destinations:\n    - \"{}\"\n",
            text(&holiday.backup)
        )
        .as_bytes(),
    );
    holiday
}

fn import(folders: &Folders, arguments: &[&str]) -> Output {
    let source = text(&folders.source());
    let mut all = vec!["import", source.as_str()];
    all.extend_from_slice(arguments);
    folders.framehive(&all)
}

fn is_empty(folder: &Path) -> bool {
    fs::read_dir(folder).unwrap().next().is_none()
}

fn assert_not_started(output: &Output, error: &str) {
    assert_eq!(stderr(output), format!("{error}\n"));
    assert!(stdout(output).is_empty(), "{}", stdout(output));
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-010.1
#[test]
fn a_profile_supplies_the_settings() {
    let folders = Folders::new();
    folders.add_raf();
    let holiday = holiday(&folders);

    let output = import(
        &folders,
        &[&text(&holiday.destination), "--profile", "holiday"],
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(holiday.destination.join(DATED_TARGET).is_file());
    assert!(holiday.backup.join(DATED_TARGET).is_file());
}

/// spec: CLI-010.2
#[test]
fn an_option_replaces_the_value_of_the_profile() {
    let folders = Folders::new();
    folders.add_raf();
    holiday(&folders);
    let other = empty_folder(&folders, "other");

    let output = import(
        &folders,
        &[
            &text(&other),
            "--profile",
            "holiday",
            "--template",
            "{name}.{ext}",
        ],
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(other.join("DSCF0001.RAF").is_file());
    assert!(!other.join(DATED_TARGET).exists());
}

/// spec: CLI-010.3
#[test]
fn backup_replaces_the_list_of_the_profile() {
    let folders = Folders::new();
    folders.add_raf();
    let holiday = holiday(&folders);
    let drive = empty_folder(&folders, "drive");

    let output = import(
        &folders,
        &[
            &text(&holiday.destination),
            "--profile",
            "holiday",
            "--backup",
            &text(&drive),
        ],
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(drive.join(DATED_TARGET).is_file());
    assert!(is_empty(&holiday.backup));
    let report = stdout(&output);
    assert!(report.contains(&text(&drive)), "{report}");
    assert!(!report.contains(&text(&holiday.backup)), "{report}");
}

/// spec: CLI-010.4
#[test]
fn the_first_job_code_of_the_profile_is_used() {
    let folders = Folders::new();
    folders.add_raf();
    folders.write_profile(
        "client",
        b"schema_version: 1\nnaming:\n  template: \"{job}/{name}.{ext}\"\njob_codes: [Smith wedding, Jones]\n",
    );
    let first = folders.root.path().join("first");
    let second = folders.root.path().join("second");

    let output = import(
        &folders,
        &["--destination", &text(&first), "--profile", "client"],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let output = import(
        &folders,
        &[
            "--destination",
            &text(&second),
            "--profile",
            "client",
            "--job-code",
            "Jones",
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));

    assert!(first.join("Smith wedding/DSCF0001.RAF").is_file());
    assert!(second.join("Jones/DSCF0001.RAF").is_file());
}

/// spec: CLI-010.5
#[test]
fn unused_job_codes_of_a_profile_give_no_warning() {
    let folders = Folders::new();
    folders.add_raf();
    folders.write_profile("client", b"schema_version: 1\njob_codes: [Smith wedding]\n");

    let output = import(
        &folders,
        &[
            "--destination",
            &text(&folders.destination()),
            "--profile",
            "client",
        ],
    );

    assert_eq!(stderr(&output), "");
    assert_eq!(output.status.code(), Some(0));
    assert!(folders.destination().join(DEFAULT_TARGET).is_file());
}

/// spec: CLI-010.6
/// spec: CLI-016.13
#[test]
fn a_profile_with_a_destination_is_rejected() {
    let folders = Folders::new();
    folders.add_raf();
    let path = folders.write_profile("holiday", b"schema_version: 1\ndestination: ~/Pictures\n");

    let output = import(
        &folders,
        &[&text(&folders.destination()), "--profile", "holiday"],
    );

    assert_not_started(
        &output,
        &format!(
            "error: {}:2:1: destination is not read from a profile; give the folder to import into on the command line",
            path.display()
        ),
    );
    assert!(!folders.destination().exists());
}

/// spec: CLI-010.7
#[test]
fn an_unknown_profile_names_the_known_ones() {
    let folders = Folders::new();
    folders.add_raf();
    folders.write_profile("holiday", b"schema_version: 1\n");
    folders.write_profile("client", b"schema_version: 1\n");
    // Neither is a profile: an invalid name, and not `.yaml`.
    folders.write_profile("Big", b"schema_version: 1\n");
    fs::write(folders.profiles_folder().join("notes.txt"), "").unwrap();

    let output = import(
        &folders,
        &[&text(&folders.destination()), "--profile", "vacation"],
    );

    assert_not_started(
        &output,
        &format!(
            "error: no profile vacation in {}; known profiles: client, holiday",
            text(&folders.profiles_folder())
        ),
    );
}

/// spec: CLI-010.8
#[test]
fn without_profiles_the_message_points_to_config_init() {
    let folders = Folders::new();
    folders.add_raf();
    fs::create_dir_all(folders.configuration_folder()).unwrap();

    let output = import(
        &folders,
        &[&text(&folders.destination()), "--profile", "holiday"],
    );

    assert_not_started(
        &output,
        &format!(
            "error: no profile holiday in {}; there are no profiles yet, see framehive config init",
            text(&folders.profiles_folder())
        ),
    );
}

/// spec: CLI-010.9
#[test]
fn an_invalid_profile_name_is_rejected() {
    let folders = Folders::new();
    folders.add_raf();
    // Were the names used as they are, these files would be found.
    folders.write_profile("Holiday", b"schema_version: 1\n");
    fs::write(
        folders.configuration_folder().join("holiday.yaml"),
        "schema_version: 1\n",
    )
    .unwrap();

    for name in ["../holiday", "Holiday"] {
        let output = import(
            &folders,
            &[
                "--destination",
                &text(&folders.destination()),
                "--profile",
                name,
            ],
        );

        assert_not_started(
            &output,
            &format!("error: invalid profile name {name}: use 1 to 64 of a-z, 0-9, - and _"),
        );
    }
}

/// spec: CLI-010.10
#[test]
fn an_invalid_profile_names_the_file_and_the_position() {
    let folders = Folders::new();
    folders.add_raf();
    let path = folders.write_profile("holiday", b"schema_version: 1\nvideo_destinaton: /a\n");

    let output = import(
        &folders,
        &[
            "--destination",
            &text(&folders.destination()),
            "--profile",
            "holiday",
        ],
    );

    assert_not_started(
        &output,
        &format!(
            "error: {}:2:1: unknown key video_destinaton; did you mean video_destination?",
            text(&path)
        ),
    );
    assert!(!folders.destination().exists());
}

/// spec: CLI-010.11
#[test]
fn a_profile_that_is_not_utf8_is_rejected() {
    let folders = Folders::new();
    folders.add_raf();
    let path = folders.write_profile("holiday", b"schema_version: 1\n# \xFF\n");

    let output = import(
        &folders,
        &[
            "--destination",
            &text(&folders.destination()),
            "--profile",
            "holiday",
        ],
    );

    assert_not_started(
        &output,
        &format!("error: the profile {} is not valid UTF-8", text(&path)),
    );
}

/// spec: CLI-010.14
#[test]
fn an_unknown_configuration_folder_stops_the_command() {
    let folders = Folders::new();
    folders.add_raf();
    let source = text(&folders.source());
    let destination = text(&folders.destination());

    let output = folders
        .framehive_command(&[
            "import",
            &source,
            "--destination",
            &destination,
            "--profile",
            "holiday",
        ])
        .env("HOME", "not-absolute")
        .output()
        .unwrap();

    assert_not_started(
        &output,
        "error: the configuration folder is unknown: set HOME to an absolute path",
    );
}

/// spec: CLI-010.18
#[cfg(target_os = "linux")]
#[test]
fn an_unknown_home_folder_stops_the_command_even_with_xdg_config_home() {
    let folders = Folders::new();
    folders.add_raf();
    let source = text(&folders.source());
    let destination = text(&folders.destination());
    let config_home = folders.root.path().join("config");

    let output = folders
        .framehive_command(&[
            "import",
            &source,
            "--destination",
            &destination,
            "--profile",
            "holiday",
        ])
        .env_remove("HOME")
        .env("XDG_CONFIG_HOME", &config_home)
        .output()
        .unwrap();

    assert_not_started(
        &output,
        "error: the home folder is unknown: set HOME to an absolute path",
    );
}
