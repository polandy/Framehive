// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import camera:…` against the real binary (CLI-013): a folder served as a
//! camera by libgphoto2's `directory` driver on a `disk:` port, a folder served by its
//! `vusb` virtual PTP camera, and a folder whose name starts with `camera:`. The child
//! process runs with `TZ=UTC`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use metadata::testing::raf;
use support::{DEFAULT_TARGET, Folders, has_line, photo, stderr, stdout};

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

/// spec: CLI-013.5
#[test]
fn camera_alone_without_a_camera_stops() {
    let folders = Folders::new();
    let iolibs = folders.root.path().join("iolibs");
    fs::create_dir(&iolibs).unwrap();
    std::os::unix::fs::symlink(
        default_iolib_folder().join("ptpip.so"),
        iolibs.join("ptpip.so"),
    )
    .unwrap();
    let destination = folders.destination();

    let output = import_from_virtual_camera(&folders, &iolibs, &destination);

    assert!(
        stderr(&output).contains("no camera is connected"),
        "{}",
        stderr(&output)
    );
    assert!(!destination.exists());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-013.10
#[test]
fn a_port_that_cannot_be_opened_stops() {
    let folders = Folders::new();
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    let destination = folders.destination();

    let output = import_from(
        &folders,
        "camera:usb:009,009",
        &iolib_folder("vusb"),
        &destination,
    );

    assert!(
        stderr(&output).contains("cannot open the camera at usb:009,009: "),
        "{}",
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(2));
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

/// spec: CLI-013.8
#[test]
fn an_unknown_model_stops() {
    let folders = Folders::new();
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    let destination = folders.destination();

    let output = import_from(
        &folders,
        "camera:Canon",
        &iolib_folder("vusb"),
        &destination,
    );

    assert!(
        stderr(&output).contains("no camera Canon is connected"),
        "{}",
        stderr(&output)
    );
    assert!(!destination.exists());
    assert_eq!(output.status.code(), Some(2));
}

/// Runs `framehive import camera:` with libgphoto2's port drivers loaded from `iolibs`
/// only, so no real USB device is seen; `vusb` serves the source folder as a Nikon D750.
fn import_from_virtual_camera(folders: &Folders, iolibs: &Path, destination: &Path) -> Output {
    import_from(folders, "camera:", iolibs, destination)
}

/// Like [`import_from_virtual_camera`], with the camera source given.
fn import_from(folders: &Folders, source: &str, iolibs: &Path, destination: &Path) -> Output {
    folders
        .framehive_command(&[
            "import",
            source,
            "--destination",
            destination.to_str().unwrap(),
        ])
        .env("IOLIBS", iolibs)
        .env("VCAMERADIR", folders.source())
        .current_dir(folders.root.path())
        .output()
        .unwrap()
}

/// A folder below the port drivers of the Nix shell's libgphoto2 (`flake.nix`).
fn iolib_folder(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os("FRAMEHIVE_TEST_IOLIBS").unwrap()).join(name)
}

/// The versioned folder libgphoto2 loads its port drivers from by default.
fn default_iolib_folder() -> PathBuf {
    let root = iolib_folder("");
    fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.file_name().is_some_and(|name| name != "vusb"))
        .unwrap()
}
