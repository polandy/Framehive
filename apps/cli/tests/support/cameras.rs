// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Runs of `framehive import camera:…` against libgphoto2's port drivers in a chosen folder.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::{Folders, iolib_folder};

/// Runs `framehive import camera:` with libgphoto2's port drivers loaded from `iolibs`
/// only, so no real USB device is seen; `vusb` serves the source folder as a Nikon D750.
pub(crate) fn import_from_virtual_camera(
    folders: &Folders,
    iolibs: &Path,
    destination: &Path,
) -> Output {
    import_from(folders, "camera:", iolibs, destination)
}

/// Like [`import_from_virtual_camera`], with the camera source given.
pub(crate) fn import_from(
    folders: &Folders,
    source: &str,
    iolibs: &Path,
    destination: &Path,
) -> Output {
    import_command(folders, source, iolibs, destination)
        .output()
        .unwrap()
}

/// The command [`import_from`] runs.
pub(crate) fn import_command(
    folders: &Folders,
    source: &str,
    iolibs: &Path,
    destination: &Path,
) -> Command {
    let mut command = folders.framehive_command(&[
        "import",
        source,
        "--destination",
        destination.to_str().unwrap(),
    ]);
    command
        .env("IOLIBS", iolibs)
        .env("VCAMERADIR", folders.source())
        .current_dir(folders.root.path());
    command
}

/// A port driver folder with only the network driver, so no camera is found.
pub(crate) fn iolibs_without_a_camera(folders: &Folders) -> PathBuf {
    let iolibs = folders.root.path().join("iolibs");
    fs::create_dir(&iolibs).unwrap();
    std::os::unix::fs::symlink(
        default_iolib_folder().join("ptpip.so"),
        iolibs.join("ptpip.so"),
    )
    .unwrap();
    iolibs
}

/// The versioned folder libgphoto2 loads its port drivers from by default.
pub(crate) fn default_iolib_folder() -> PathBuf {
    let root = iolib_folder("");
    fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.file_name().is_some_and(|name| name != "vusb"))
        .unwrap()
}
