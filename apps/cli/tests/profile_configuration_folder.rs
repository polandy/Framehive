// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Where `framehive import --profile` looks for the profile, against the real binary
//! (CLI-010.12, CLI-010.13).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::Path;
use std::process::Output;

use support::{Folders, stderr};

const PROFILE: &[u8] = b"schema_version: 1\nnaming:\n  template: \"{name}.{ext}\"\n";

fn write_profile_in(configuration_folder: &Path) {
    let profiles = configuration_folder.join("framehive/profiles");
    fs::create_dir_all(&profiles).unwrap();
    fs::write(profiles.join("holiday.yaml"), PROFILE).unwrap();
}

/// Imports the source with the profile `holiday` into a fresh library, with the
/// `XDG_CONFIG_HOME` given and `HOME` set to the home folder of the test.
fn import_with_profile(folders: &Folders, library: &str, xdg_config_home: Option<&Path>) -> Output {
    let destination = folders.root.path().join(library);
    fs::create_dir_all(&destination).unwrap();
    let mut command = folders.framehive_command(&[
        "import",
        &folders.source().display().to_string(),
        &destination.display().to_string(),
        "--profile",
        "holiday",
    ]);
    if let Some(folder) = xdg_config_home {
        command.env("XDG_CONFIG_HOME", folder);
    }
    command.output().unwrap()
}

fn assert_imported(folders: &Folders, library: &str, output: &Output) {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(
        folders
            .root
            .path()
            .join(library)
            .join("DSCF0001.RAF")
            .is_file()
    );
}

/// spec: CLI-010.12
#[cfg(target_os = "linux")]
#[test]
fn linux_finds_the_profile_in_xdg_config_home_and_else_in_the_home_folder() {
    let folders = Folders::new();
    folders.add_raf();
    let xdg = folders.root.path().join("xdg");
    write_profile_in(&xdg);

    let with_xdg = import_with_profile(&folders, "with-xdg", Some(&xdg));
    assert_imported(&folders, "with-xdg", &with_xdg);

    write_profile_in(&folders.home().join(".config"));
    folders.forget_history();
    let without_xdg = import_with_profile(&folders, "without-xdg", None);
    assert_imported(&folders, "without-xdg", &without_xdg);
}

/// spec: CLI-010.13
#[cfg(target_os = "macos")]
#[test]
fn macos_finds_the_profile_in_application_support_and_ignores_xdg_config_home() {
    let folders = Folders::new();
    folders.add_raf();
    write_profile_in(&folders.home().join("Library/Application Support"));
    let other = folders.root.path().join("other");
    fs::create_dir_all(&other).unwrap();

    let output = import_with_profile(&folders, "library", Some(&other));

    assert_imported(&folders, "library", &output);
}
