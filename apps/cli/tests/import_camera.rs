// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import camera:…` against the real binary (CLI-013): a folder served as a
//! camera by libgphoto2's `directory` driver on a `disk:` port, a folder served by its
//! `vusb` virtual PTP camera, and a folder whose name starts with `camera:`. The child
//! process runs with `TZ=UTC`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::process::Output;

use metadata::testing::raf;
use support::cameras::{import_from, import_from_virtual_camera, iolibs_without_a_camera};
use support::{DEFAULT_TARGET, Folders, has_line, iolib_folder, photo, stderr, stdout};

/// spec: CLI-013.1
#[test]
fn the_virtual_camera_is_listed() {
    let folders = Folders::new();

    let output = folders
        .framehive_command(&["cameras"])
        .env("IOLIBS", iolib_folder("vusb"))
        .env("VCAMERADIR", folders.source())
        .current_dir(folders.root.path())
        .output()
        .unwrap();

    assert_eq!(stdout(&output), "Nikon DSC D750  usb:001,001\n");
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-013.2
#[test]
fn without_a_camera_the_list_is_empty() {
    let folders = Folders::new();
    let iolibs = iolibs_without_a_camera(&folders);

    let output = folders
        .framehive_command(&["cameras"])
        .env("IOLIBS", &iolibs)
        .env("VCAMERADIR", folders.source())
        .current_dir(folders.root.path())
        .output()
        .unwrap();

    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).starts_with("no camera is connected\nhint: "),
        "{}",
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(0));
}

/// spec: CLI-013.3
#[test]
fn a_camera_given_by_its_port_is_imported() {
    let folders = Folders::new();
    folders.add("store_00010001/DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    let source = format!("camera:disk:{}", folders.source().display());
    let destination = folders.destination();

    let output = folders.framehive(&[
        "import",
        &source,
        "--destination",
        destination.to_str().unwrap(),
    ]);

    assert!(
        has_line(
            &output,
            &format!("copied  store_00010001/DCIM/100_FUJI/DSCF0001.RAF -> {DEFAULT_TARGET}")
        ),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(destination.join(DEFAULT_TARGET).is_file());
    assert_eq!(output.status.code(), Some(0));
}

/// spec: CLI-013.13
#[test]
fn a_folder_named_like_a_camera_is_imported_as_a_folder() {
    let folders = Folders::new();
    let card = folders.root.path().join("camera:x");
    fs::create_dir(&card).unwrap();
    fs::write(card.join("DSCF0001.RAF"), raf(&photo())).unwrap();
    let destination = folders.destination();

    let output = folders
        .framehive_command(&[
            "import",
            "./camera:x",
            "--destination",
            destination.to_str().unwrap(),
        ])
        .current_dir(folders.root.path())
        .output()
        .unwrap();

    assert!(
        has_line(
            &output,
            &format!("copied  DSCF0001.RAF -> {DEFAULT_TARGET}")
        ),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(0));
}

/// spec: CLI-013.3
#[test]
fn the_virtual_camera_is_imported_by_its_port() {
    let folders = Folders::new();
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    let destination = folders.destination();

    let output = import_from(
        &folders,
        "camera:usb:001,001",
        &iolib_folder("vusb"),
        &destination,
    );

    assert!(
        has_line(
            &output,
            &format!("copied  store_00010001/DCIM/100_FUJI/DSCF0001.RAF -> {DEFAULT_TARGET}")
        ),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(destination.join(DEFAULT_TARGET).is_file());
    assert_eq!(output.status.code(), Some(0));
}

/// spec: CLI-013.4
#[test]
fn camera_alone_takes_the_only_camera() {
    let folders = Folders::new();
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    let destination = folders.destination();

    let output = import_from_virtual_camera(&folders, &iolib_folder("vusb"), &destination);

    assert!(
        has_line(
            &output,
            &format!("copied  store_00010001/DCIM/100_FUJI/DSCF0001.RAF -> {DEFAULT_TARGET}")
        ),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(destination.join(DEFAULT_TARGET).is_file());
    assert_eq!(output.status.code(), Some(0));
}

/// spec: CLI-013.7
#[test]
fn a_camera_is_chosen_by_its_model_ignoring_case() {
    let folders = Folders::new();
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    let destination = folders.destination();

    let output = import_from(
        &folders,
        "camera:nikon dsc d750",
        &iolib_folder("vusb"),
        &destination,
    );

    assert!(
        has_line(
            &output,
            &format!("copied  store_00010001/DCIM/100_FUJI/DSCF0001.RAF -> {DEFAULT_TARGET}")
        ),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(0));
}

/// spec: CLI-013.12
#[test]
fn the_import_log_names_the_camera() {
    let folders = Folders::new();
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    let destination = folders.destination();
    let import = import_from_virtual_camera(&folders, &iolib_folder("vusb"), &destination);
    assert_eq!(import.status.code(), Some(0), "{}", stderr(&import));

    let output = folders.import_log(&["1"]);

    assert!(
        has_line(&output, "source               camera:Nikon DSC D750"),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
}

/// spec: CLI-013.14
#[test]
fn a_camera_run_notes_that_only_dcim_was_checked() {
    let folders = Folders::new();
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    let destination = folders.destination();

    let output = import_from_virtual_camera(&folders, &iolib_folder("vusb"), &destination);

    let lines = lines_of(&output);
    assert_eq!(
        lines.last().map(String::as_str),
        Some(DCIM_NOTE),
        "{lines:?}"
    );
    assert!(
        lines[lines.len() - 2].starts_with("The source can be formatted: "),
        "{lines:?}"
    );
}

/// spec: CLI-013.14
#[test]
fn a_dry_run_of_a_camera_has_no_note() {
    let folders = Folders::new();
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    let destination = folders.destination();

    let output = folders
        .framehive_command(&[
            "import",
            "camera:",
            "--destination",
            destination.to_str().unwrap(),
            "--dry-run",
        ])
        .env("IOLIBS", iolib_folder("vusb"))
        .env("VCAMERADIR", folders.source())
        .current_dir(folders.root.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(!stdout(&output).contains(DCIM_NOTE), "{}", stdout(&output));
}

const DCIM_NOTE: &str = "Only the files below DCIM were checked. Files outside DCIM, such as a phone's other folders, are not backed up.";

fn lines_of(output: &Output) -> Vec<String> {
    stdout(output).lines().map(str::to_owned).collect()
}
