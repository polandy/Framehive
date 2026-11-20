// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import camera:…` against the real binary when the camera cannot be used
//! (CLI-013): no camera, an unknown model, a camera in use, a port that cannot be
//! opened, a camera without a storage, and a camera that goes away, through
//! libgphoto2's `vusb` virtual PTP camera.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use metadata::testing::raf;
use support::cameras::{
    import_command, import_from, import_from_virtual_camera, iolibs_without_a_camera,
};
use support::{DEFAULT_TARGET, Folders, has_line, iolib_folder, photo, stderr, stdout};

/// spec: CLI-013.5
#[test]
fn camera_alone_without_a_camera_stops() {
    let folders = Folders::new();
    let iolibs = iolibs_without_a_camera(&folders);
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

/// spec: CLI-013.9
#[test]
fn a_camera_in_use_stops_with_a_remedy() {
    let folders = Folders::new();
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    let destination = folders.destination();
    // The patched `vusb` driver reports its camera as claimed while the folder is locked.
    let holder = fs::File::open(folders.source()).unwrap();
    holder.try_lock().unwrap();

    let output = import_from_virtual_camera(&folders, &iolib_folder("vusb"), &destination);

    assert!(stderr(&output).contains(IN_USE), "{}", stderr(&output));
    assert!(!destination.exists());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-013.15
#[test]
fn a_camera_without_a_storage_stops_with_a_remedy() {
    let folders = Folders::new();
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    let destination = folders.destination();
    // The patched `vusb` driver lists no storage while this file exists.
    let lock = folders.root.path().join("locked");
    fs::write(&lock, "").unwrap();

    let output = import_command(&folders, "camera:", &iolib_folder("vusb"), &destination)
        .env("VCAMERA_LOCKED_WHILE", &lock)
        .output()
        .unwrap();

    assert!(
        stderr(&output).contains(
            "the camera Nikon DSC D750 at usb:001,001 shows no storage; unlock it and choose \
             file transfer (MTP or PTP) on the device, then try again"
        ),
        "{}",
        stderr(&output)
    );
    assert!(!destination.exists());
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-013.11
#[test]
fn a_camera_that_goes_away_ends_with_exit_code_1() {
    let folders = Folders::new();
    folders.add("DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    folders.add("DCIM/100_FUJI/DSCF0002.RAF", &raf(&photo()));
    let destination = folders.destination();

    // The patched `vusb` driver stops answering once the first copy exists.
    let output = import_command(&folders, "camera:", &iolib_folder("vusb"), &destination)
        .env("VCAMERA_UNPLUG_WHEN", destination.join(DEFAULT_TARGET))
        .output()
        .unwrap();

    assert!(
        has_line(
            &output,
            &format!("copied  store_00010001/DCIM/100_FUJI/DSCF0001.RAF -> {DEFAULT_TARGET}")
        ),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("failed  store_00010001/DCIM/100_FUJI/DSCF0002.RAF"),
        "{}",
        stdout(&output)
    );
    assert!(
        stdout(&output)
            .contains("Do not format the source: not every file is verified in 1 location."),
        "{}",
        stdout(&output)
    );
    assert_eq!(output.status.code(), Some(1));
}

const IN_USE: &str = "the camera Nikon DSC D750 at usb:001,001 is in use by another program; close it (the file manager on Linux, ptpcamerad on macOS) and try again";

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
