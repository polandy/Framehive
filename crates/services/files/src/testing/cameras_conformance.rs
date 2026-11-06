// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`Cameras`] port (ADR 0034 §2).

use std::path::Path;

use super::read_only_file_system_conformance::{
    AddFile, AddFolder, ReadOnlyFileSystemUnderTest, check_read_only_file_system,
};
use crate::{CameraError, Cameras, EntryKind, FolderEntry};

/// The storage folders the suite gives the camera.
const STORAGES: [&str; 2] = ["store_00010001", "store_00020001"];

/// What the [`Cameras`] conformance suite runs against.
pub struct CamerasUnderTest<'a> {
    /// The implementation under test.
    pub cameras: &'a dyn Cameras,
    /// The port of a camera that is connected, not in use and empty: its root holds
    /// no storage folder yet.
    pub port: &'a str,
    /// A port at which no camera is connected.
    pub missing_port: &'a str,
    /// Adds a folder to the camera at `port`, by its path in the camera's file system.
    pub add_folder: &'a AddFolder<'a>,
    /// Adds a file to the camera at `port`, by its path in the camera's file system.
    pub add_file: &'a AddFile<'a>,
}

/// Runs the conformance suite of the [`Cameras`] port and returns one message per
/// failed check, or nothing if the implementation conforms.
///
/// It lays out two storage folders on the camera at `port`, and runs
/// [`check_read_only_file_system`] on the opened camera inside the first one.
#[must_use]
pub fn check_cameras(under_test: &CamerasUnderTest<'_>) -> Vec<String> {
    let mut failures = Vec::new();
    check_list_is_sorted(under_test, &mut failures);
    check_missing_port(under_test, &mut failures);
    if let Err(message) = add_storages(under_test) {
        failures.push(message);
        return failures;
    }
    check_storages_are_listed(under_test, &mut failures);
    check_opened_camera_reads(under_test, &mut failures);
    check_released_camera_opens_again(under_test, &mut failures);
    failures
}

/// Cameras list in byte order of their ports, each port once.
fn check_list_is_sorted(under_test: &CamerasUnderTest<'_>, failures: &mut Vec<String>) {
    match under_test.cameras.list() {
        Ok(cameras) => {
            let ports: Vec<&str> = cameras.iter().map(|camera| camera.port.as_str()).collect();
            if !ports.windows(2).all(|pair| pair[0] < pair[1]) {
                failures.push(format!("list: ports {ports:?} are not sorted and distinct"));
            }
        }
        Err(error) => failures.push(format!("list failed: {error}")),
    }
}

fn check_missing_port(under_test: &CamerasUnderTest<'_>, failures: &mut Vec<String>) {
    match under_test.cameras.open(under_test.missing_port) {
        Ok(_) => failures.push(format!(
            "open: {} succeeded without a camera",
            under_test.missing_port
        )),
        Err(CameraError::Open { port, .. }) if port == under_test.missing_port => {}
        Err(error) => failures.push(format!(
            "open: {} failed with {error:?}, expected CameraError::Open for that port",
            under_test.missing_port
        )),
    }
}

fn add_storages(under_test: &CamerasUnderTest<'_>) -> Result<(), String> {
    for storage in STORAGES {
        (under_test.add_folder)(&Path::new("/").join(storage))
            .map_err(|error| format!("cannot add the storage {storage}: {error}"))?;
    }
    Ok(())
}

/// The root of an opened camera holds one folder per storage.
fn check_storages_are_listed(under_test: &CamerasUnderTest<'_>, failures: &mut Vec<String>) {
    let camera = match under_test.cameras.open(under_test.port) {
        Ok(camera) => camera,
        Err(error) => return failures.push(format!("open failed: {error}")),
    };
    let root = Path::new("/");
    match camera.entry(root) {
        Ok(Some(EntryKind::Folder)) => {}
        other => failures.push(format!("entry of / is {other:?}, expected a folder")),
    }
    match camera.list_folder(root) {
        Ok(mut entries) => {
            entries.sort_by(|a, b| a.name.cmp(&b.name));
            let expected: Vec<FolderEntry> = STORAGES
                .iter()
                .map(|name| FolderEntry {
                    name: (*name).into(),
                    kind: EntryKind::Folder,
                })
                .collect();
            if entries != expected {
                failures.push(format!("/ lists {entries:?}, expected {expected:?}"));
            }
        }
        Err(error) => failures.push(format!("list_folder of / failed: {error}")),
    }
}

/// An opened camera is a conforming [`ReadOnlyFileSystem`](crate::ReadOnlyFileSystem).
fn check_opened_camera_reads(under_test: &CamerasUnderTest<'_>, failures: &mut Vec<String>) {
    let camera = match under_test.cameras.open(under_test.port) {
        Ok(camera) => camera,
        Err(error) => return failures.push(format!("open failed: {error}")),
    };
    let root = Path::new("/").join(STORAGES[0]).join("read_only");
    if let Err(error) = (under_test.add_folder)(&root) {
        return failures.push(format!("cannot add {}: {error}", root.display()));
    }
    let read_only = check_read_only_file_system(&ReadOnlyFileSystemUnderTest {
        file_system: camera.as_ref(),
        root: &root,
        add_folder: under_test.add_folder,
        add_file: under_test.add_file,
        make_symbolic_link: None,
    });
    failures.extend(
        read_only
            .into_iter()
            .map(|failure| format!("opened camera: {failure}")),
    );
}

/// Dropping an opened camera releases it, so it can be opened again.
fn check_released_camera_opens_again(
    under_test: &CamerasUnderTest<'_>,
    failures: &mut Vec<String>,
) {
    for attempt in ["first", "second"] {
        match under_test.cameras.open(under_test.port) {
            Ok(camera) => drop(camera),
            Err(error) => {
                return failures.push(format!("open after release ({attempt}) failed: {error}"));
            }
        }
    }
}
