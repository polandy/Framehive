// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import camera:…` with cameras in memory (CLI-013): the process tests
//! cannot connect a camera.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::Path;

use domain::Timestamp;
use files::testing::{MemoryCameras, MemoryFileSystem};
use import::CAMERA_FORMAT_READINESS_NOTE;

use super::tests::{Folders, Run};
use crate::arguments::{ImportLogArguments, LogFormat, RunSelection};
use crate::import_log::import_log_in;

const X_T2: (&str, &str) = ("Fujifilm X-T2", "usb:001,004");
const X_T5: (&str, &str) = ("Fujifilm X-T5", "usb:002,007");
const FIRST: &str = "store_00010001/DCIM/100_FUJI/DSCF0001.RAF";
const SECOND: &str = "store_00010001/DCIM/100_FUJI/DSCF0002.RAF";
/// 2026-10-17 14:05:09 UTC.
const MODIFIED: i64 = 1_792_245_909;

/// A camera file system holding `files`, given relative to its root.
fn camera_with(files: &[&str]) -> MemoryFileSystem {
    let camera = MemoryFileSystem::new();
    for file in files {
        camera.add_file(
            &Path::new("/").join(file),
            file.as_bytes(),
            Timestamp::from_unix_seconds(MODIFIED),
        );
    }
    camera
}

/// One connected camera `Fujifilm X-T5` with `DSCF0001.RAF`, and its file system.
fn one_camera() -> (MemoryCameras, MemoryFileSystem) {
    let camera = camera_with(&[FIRST]);
    let (model, port) = X_T5;
    (
        MemoryCameras::new().with_camera(model, port, &camera),
        camera,
    )
}

/// The cameras of CLI-013.1, each holding a file named after its model.
fn two_cameras() -> MemoryCameras {
    MemoryCameras::new()
        .with_camera(X_T2.0, X_T2.1, &camera_with(&["store/DCIM/X_T2.RAF"]))
        .with_camera(X_T5.0, X_T5.1, &camera_with(&["store/DCIM/X_T5.RAF"]))
}

fn assert_copied(run: &Run, source_path: &str) {
    let line = format!("copied  {source_path} -> ");
    assert!(
        run.output.lines().any(|output| output.starts_with(&line)),
        "missing `{line}` in:\n{}{}",
        run.output,
        run.errors
    );
}

/// Asserts the import stopped with `message` and created nothing.
fn assert_stopped(folders: &Folders, run: &Run, message: &str) {
    assert_eq!(run.not_started(), [message.to_owned()]);
    assert!(!folders.path("library").exists());
}

/// spec: CLI-013.4
#[test]
fn camera_takes_the_only_camera_and_releases_it() {
    let (folders, (cameras, _camera)) = (Folders::new(), one_camera());

    let run = folders.import_camera("camera:", &[], &cameras);

    assert_copied(&run, FIRST);
    assert!(matches!(&run.result, Ok(summary) if !summary.has_failures()));
    assert!(!cameras.is_open(X_T5.1));
}

/// spec: CLI-013.5
#[test]
fn camera_without_a_camera_stops() {
    let folders = Folders::new();

    let run = folders.import_camera("camera:", &[], &MemoryCameras::new());

    assert_stopped(&folders, &run, "no camera is connected");
}

/// spec: CLI-013.6
#[test]
fn camera_with_several_cameras_stops() {
    let folders = Folders::new();

    let run = folders.import_camera("camera:", &[], &two_cameras());

    assert_stopped(
        &folders,
        &run,
        "several cameras are connected; name one: camera:usb:001,004 (Fujifilm X-T2), \
         camera:usb:002,007 (Fujifilm X-T5)",
    );
}

/// spec: CLI-013.7
#[test]
fn a_camera_is_chosen_by_its_model_ignoring_case() {
    let folders = Folders::new();

    let run = folders.import_camera("camera:fujifilm x-t5", &[], &two_cameras());

    assert_copied(&run, "store/DCIM/X_T5.RAF");
    assert!(!run.output.contains("X_T2.RAF"), "{}", run.output);
}

/// spec: CLI-013.8
#[test]
fn an_unknown_model_stops() {
    let folders = Folders::new();

    let run = folders.import_camera("camera:Canon", &[], &two_cameras());

    assert_stopped(&folders, &run, "no camera Canon is connected");
}

/// spec: CLI-013.8
#[test]
fn an_ambiguous_model_stops() {
    let folders = Folders::new();
    let camera = camera_with(&[FIRST]);
    let cameras = MemoryCameras::new()
        .with_camera(X_T5.0, X_T2.1, &camera)
        .with_camera(X_T5.0, X_T5.1, &camera);

    let run = folders.import_camera("camera:Fujifilm X-T5", &[], &cameras);

    assert_stopped(
        &folders,
        &run,
        "several cameras Fujifilm X-T5 are connected; name one: camera:usb:001,004, \
         camera:usb:002,007",
    );
}

/// spec: CLI-013.9
#[test]
fn a_camera_in_use_stops_with_a_remedy() {
    let folders = Folders::new();
    let cameras = MemoryCameras::new().with_camera_in_use(X_T5.0, X_T5.1, "device busy");

    let run = folders.import_camera("camera:", &[], &cameras);

    assert_stopped(
        &folders,
        &run,
        "the camera Fujifilm X-T5 at usb:002,007 is in use by another program; close it \
         (the file manager on Linux, ptpcamerad on macOS) and try again",
    );
}

/// spec: CLI-013.15
#[test]
fn a_camera_without_a_storage_stops_with_a_remedy() {
    let folders = Folders::new();
    let cameras = MemoryCameras::new().with_camera(X_T5.0, X_T5.1, &MemoryFileSystem::new());

    let run = folders.import_camera("camera:", &[], &cameras);

    assert_stopped(
        &folders,
        &run,
        "the camera Fujifilm X-T5 at usb:002,007 shows no storage; unlock it and choose \
         file transfer (MTP or PTP) on the device, then try again",
    );
}

/// spec: CLI-013.10
#[test]
fn a_port_that_cannot_be_opened_stops() {
    let folders = Folders::new();

    let run = folders.import_camera("camera:usb:009,009", &[], &MemoryCameras::new());

    let [message] = run.not_started() else {
        panic!("{:?}", run.not_started());
    };
    assert!(
        message.starts_with("cannot open the camera at usb:009,009: "),
        "{message}"
    );
    assert!(!folders.path("library").exists());
}

/// spec: CLI-013.11
#[test]
fn a_camera_that_goes_away_fails_the_remaining_files() {
    // The camera stops answering after a number of its operations. The smallest
    // number that lets the first file be copied stops it before the second.
    let run = (0..200)
        .map(|operations| {
            let camera = camera_with(&[FIRST, SECOND]);
            let cameras = MemoryCameras::new().with_camera(X_T5.0, X_T5.1, &camera);
            camera.stop_after_operations(operations);
            Folders::new().import_camera("camera:", &[], &cameras)
        })
        .find(|run| run.output.contains(&format!("copied  {FIRST} -> ")))
        .unwrap();

    assert!(
        run.output.contains(&format!("failed  {SECOND}")),
        "{}",
        run.output
    );
    assert!(
        run.output
            .contains("Do not format the source: not every file is verified in 1 location."),
        "{}",
        run.output
    );
    assert!(matches!(&run.result, Ok(summary) if summary.has_failures()));
}

/// spec: CLI-013.12
#[test]
fn the_import_log_names_the_camera() {
    let (folders, (cameras, _camera)) = (Folders::new(), one_camera());
    folders.import_camera("camera:", &[], &cameras);

    let log = import_log_in(
        &folders.path("state"),
        ImportLogArguments {
            run: Some(RunSelection::Number(1)),
            format: LogFormat::Text,
        },
    )
    .unwrap();

    assert!(
        log.lines()
            .any(|line| line == "source               camera:Fujifilm X-T5"),
        "{log}"
    );
}

/// spec: CLI-013.14
#[test]
fn a_camera_run_notes_that_only_dcim_was_checked() {
    let (folders, (cameras, _camera)) = (Folders::new(), one_camera());

    let run = folders.import_camera("camera:", &[], &cameras);

    let lines: Vec<&str> = run.output.lines().collect();
    assert_eq!(
        lines.last(),
        Some(&CAMERA_FORMAT_READINESS_NOTE),
        "{}",
        run.output
    );
    assert!(
        lines[lines.len() - 2].starts_with("The source can be formatted: "),
        "{}",
        run.output
    );
}

/// spec: CLI-013.14
#[test]
fn a_dry_run_of_a_camera_has_no_note() {
    let (folders, (cameras, _camera)) = (Folders::new(), one_camera());

    let run = folders.import_camera("camera:", &["--dry-run"], &cameras);

    assert!(
        !run.output.contains(CAMERA_FORMAT_READINESS_NOTE),
        "{}",
        run.output
    );
}

#[test]
fn a_camera_takes_a_backup_destination() {
    let (folders, (cameras, _camera)) = (Folders::new(), one_camera());
    let backup = folders.path("backup");

    let run = folders.import_camera("camera:", &["--backup", backup.to_str().unwrap()], &cameras);

    assert!(
        matches!(&run.result, Ok(summary) if !summary.has_failures()),
        "{}",
        run.output
    );
    assert!(backup.exists());
}
