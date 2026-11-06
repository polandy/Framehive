// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cameras in the guided import (CLI-014): waiting, opening, importing, releasing and a
//! camera that goes during the import.

use std::path::{Path, PathBuf};
use std::time::Duration;

use domain::Timestamp;
use files::testing::{MemoryCameras, MemoryFileSystem};

use super::support::{Harness, OCTOBER_17, PICTURES, PRESET_1, Step};

const MODEL: &str = "Fujifilm X-T5";
const PORT: &str = "usb:002,007";
const OTHER_PORT: &str = "usb:002,008";
const WAITING: &str = "Waiting for a memory card or camera…\n";
const IN_USE: &str = "The camera Fujifilm X-T5 at usb:002,007 is in use by another program; \
                      close it (the file manager on Linux, ptpcamerad on macOS) and try again.\n\
                      Try again? [Y/n]: ";
const NO_STORAGE: &str = "The camera Fujifilm X-T5 at usb:002,007 shows no storage; unlock it \
                          and choose file transfer (MTP or PTP) on the device, then try again.\n\
                          Try again? [Y/n]: ";
const REMOVED: &str = "[REMOVED] Fujifilm X-T5\n\
                       Fujifilm X-T5 was disconnected during the import. Connect it again to go on.\n";

/// A harness with a remembered destination and preset 1 whose camera holds `photos`
/// photos below `DCIM`, not yet connected.
fn with_photos(photos: u8) -> Harness {
    let harness = Harness::new();
    harness.remember(Some(PICTURES), Some(PRESET_1), &[]);
    add_photos(&harness.camera_storage, photos);
    harness
}

/// Adds `photos` photos below `DCIM` of one storage of `storage`.
fn add_photos(storage: &MemoryFileSystem, photos: u8) {
    storage.add_folder(Path::new("/store_00010001/DCIM"));
    for number in 1..=photos {
        storage.add_file(
            &photo(number),
            &[number; 1000],
            Timestamp::from_unix_seconds(OCTOBER_17),
        );
    }
}

fn photo(number: u8) -> PathBuf {
    PathBuf::from(format!("/store_00010001/DCIM/100_FUJI/DSCF000{number}.RAF"))
}

fn imported(number: u8) -> PathBuf {
    Path::new(PICTURES).join(format!("2026/2026-10-17/DSCF000{number}.RAF"))
}

/// Changes the connected cameras of `harness`.
fn change_cameras(harness: &Harness, change: impl FnOnce(MemoryCameras) -> MemoryCameras) {
    let cameras = harness.cameras.take();
    harness.cameras.replace(change(cameras));
}

/// Connects the camera of `harness` at [`PORT`].
fn connect(harness: &Harness) {
    change_cameras(harness, |cameras| {
        cameras.with_camera(MODEL, PORT, &harness.camera_storage)
    });
}

/// Connects the camera of `harness` at [`PORT`], held by another program.
fn connect_in_use(harness: &Harness) {
    change_cameras(harness, |cameras| {
        cameras.with_camera_in_use(MODEL, PORT, "Could not claim the USB device")
    });
}

/// Disconnects the camera at [`PORT`].
fn disconnect(harness: &Harness) {
    change_cameras(harness, |cameras| cameras.without_camera(PORT));
}

/// spec: CLI-014.1
#[test]
fn a_camera_appears_and_goes() {
    let harness = with_photos(1);

    let transcript = harness.run(vec![
        Step::Tick,
        Step::Call(connect),
        Step::Tick,
        Step::Call(disconnect),
        Step::Tick,
    ]);

    let output = &transcript.output;
    let waiting = output.find(WAITING).unwrap();
    let new = output
        .find("[NEW] Fujifilm X-T5 (camera at usb:002,007)\nReading Fujifilm X-T5…\n")
        .unwrap();
    let removed = output.find("[REMOVED] Fujifilm X-T5\n").unwrap();
    assert!(waiting < new && new < removed, "{output}");
    assert!(output[removed..].contains(WAITING), "{output}");
}

/// spec: CLI-014.2
#[test]
fn a_card_and_a_camera_are_offered_together() {
    let harness = with_photos(1);
    harness.insert("EOS_DIGITAL");
    connect(&harness);

    let transcript = harness.run(vec![Step::Line("2")]);

    assert!(
        transcript.output.contains(
            "Which card or camera?\n  \
             1  EOS_DIGITAL (/run/media/ada/EOS_DIGITAL)\n  \
             2  Fujifilm X-T5 (camera at usb:002,007)\n"
        ),
        "{}",
        transcript.output
    );
    assert!(
        transcript.output.contains("2\nReading Fujifilm X-T5…\n"),
        "{}",
        transcript.output
    );
}

/// spec: CLI-014.3
#[test]
fn a_camera_is_imported() {
    let harness = with_photos(2);
    connect(&harness);

    let transcript = harness.run(vec![Step::Line(""), Step::Line("y")]);

    let output = &transcript.output;
    assert!(
        output.contains(
            "2 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n\
             The camera can be formatted: every file is verified in 1 location.\n\
             Only the files below DCIM were checked. Files outside DCIM, such as a phone's \
             other folders, are not backed up.\n"
        ),
        "{output}"
    );
    assert!(!output.contains("on the card"), "{output}");
    for number in 1..=2 {
        assert_eq!(
            harness.memory.content(&imported(number)),
            Some(vec![number; 1000])
        );
    }
    let runs = harness.history.list_runs().unwrap();
    assert_eq!(runs[0].start.source, Path::new("camera:Fujifilm X-T5"));
}

/// spec: CLI-014.4
#[test]
fn a_camera_in_use_is_tried_again() {
    let harness = with_photos(1);
    connect_in_use(&harness);

    let transcript = harness.run(vec![
        Step::Call(connect),
        Step::Line("y"),
        Step::Line(""),
        Step::Line("y"),
    ]);

    let output = &transcript.output;
    assert!(
        output.contains(&format!("{IN_USE}y\nReading Fujifilm X-T5…\n")),
        "{output}"
    );
    assert!(output.contains("1 copied"), "{output}");
}

/// spec: CLI-014.5
#[test]
fn a_camera_in_use_is_not_offered_again_until_reconnected() {
    let harness = with_photos(1);
    connect_in_use(&harness);

    let transcript = harness.run(vec![
        Step::Line("n"),
        Step::Tick,
        Step::Tick,
        Step::Call(disconnect),
        Step::Tick,
        Step::Call(connect),
        Step::Tick,
    ]);

    let output = &transcript.output;
    let refused = output.find(&format!("{IN_USE}n\n")).unwrap();
    let removed = output.find("[REMOVED] Fujifilm X-T5\n").unwrap();
    assert!(refused < removed, "{output}");
    assert!(
        !output[refused..removed].contains("Reading Fujifilm X-T5…"),
        "{output}"
    );
    assert!(
        output[removed..]
            .contains("[NEW] Fujifilm X-T5 (camera at usb:002,007)\nReading Fujifilm X-T5…\n"),
        "{output}"
    );
}

/// spec: CLI-014.6
#[test]
fn a_camera_without_storage_is_tried_again() {
    let harness = Harness::new();
    harness.remember(Some(PICTURES), Some(PRESET_1), &[]);
    connect(&harness);

    let transcript = harness.run(vec![
        Step::Call(|harness| add_photos(&harness.camera_storage, 1)),
        Step::Line("y"),
        Step::Line(""),
        Step::Line("y"),
    ]);

    let output = &transcript.output;
    assert!(output.contains(&format!("{NO_STORAGE}y\n")), "{output}");
    assert!(output.contains("1 copied"), "{output}");
}

/// spec: CLI-014.7
#[test]
fn a_released_camera_is_closed_and_not_offered_again() {
    let harness = with_photos(1);
    connect(&harness);

    let transcript = harness.run(vec![
        Step::Line(""),
        Step::Line("y"),
        Step::Tick,
        Step::Tick,
    ]);

    let output = &transcript.output;
    let released = output
        .find(&format!(
            "Release Fujifilm X-T5? [Y/n]: y\nFujifilm X-T5 can be disconnected now.\n{WAITING}"
        ))
        .unwrap();
    assert!(
        !output[released..].contains("Reading Fujifilm X-T5…"),
        "{output}"
    );
    assert!(!harness.cameras.borrow().is_open(PORT));
}

/// spec: CLI-014.8
#[test]
fn a_camera_kept_open_stays_open_until_disconnected() {
    let harness = with_photos(1);
    connect(&harness);

    let transcript = harness.run(vec![
        Step::Line(""),
        Step::Line("n"),
        Step::Tick,
        Step::Call(|harness| assert!(harness.cameras.borrow().is_open(PORT))),
        Step::Tick,
        Step::Call(disconnect),
        Step::Tick,
        Step::Call(|harness| assert!(!harness.cameras.borrow().is_open(PORT))),
    ]);

    let output = &transcript.output;
    let kept = output
        .find(&format!("Release Fujifilm X-T5? [Y/n]: n\n{WAITING}"))
        .unwrap();
    assert!(
        !output[kept..].contains("Reading Fujifilm X-T5…"),
        "{output}"
    );
    assert!(
        output[kept..].contains("[REMOVED] Fujifilm X-T5\n"),
        "{output}"
    );
}

/// Disconnects the camera at [`PORT`] and makes its storage stop answering.
fn disconnect_during_import(harness: &Harness) {
    disconnect(harness);
    harness.camera_storage.stop_after_operations(0);
}

/// Connects the camera at [`OTHER_PORT`], with a storage that answers again.
fn reconnect_elsewhere(harness: &Harness) {
    let storage = MemoryFileSystem::new();
    add_photos(&storage, 3);
    change_cameras(harness, |cameras| {
        cameras.with_camera(MODEL, OTHER_PORT, &storage)
    });
}

/// spec: CLI-014.9
#[test]
fn a_camera_disconnected_during_the_import_is_resumed() {
    let harness = with_photos(3);
    harness.time_per_copy.set(Duration::from_secs(1));
    harness
        .after_copies
        .set(Some((1, disconnect_during_import)));
    connect(&harness);

    let transcript = harness.run(vec![
        Step::Line(""),
        Step::Tick,
        Step::Call(reconnect_elsewhere),
        Step::Tick,
        Step::Line("y"),
    ]);

    let output = &transcript.output;
    let removed = output.find(REMOVED).unwrap();
    let resumed = &output[removed..];
    assert!(
        resumed.contains("[NEW] Fujifilm X-T5 (camera at usb:002,008)\nReading Fujifilm X-T5…\n"),
        "{output}"
    );
    assert!(resumed.contains("2 copied, 1 already imported"), "{output}");
    assert!(!resumed.contains("[Enter] import"), "{output}");
    for number in 1..=3 {
        assert_eq!(
            harness.memory.content(&imported(number)),
            Some(vec![number; 1000])
        );
    }
}

/// spec: CLI-014.10
#[test]
fn a_camera_that_stops_answering_stops_the_import() {
    let harness = with_photos(3);
    harness.time_per_copy.set(Duration::from_secs(1));
    harness.after_copies.set(Some((1, |harness| {
        harness.camera_storage.stop_after_operations(0);
    })));
    connect(&harness);

    let transcript = harness.run(vec![
        Step::Line(""),
        Step::Tick,
        Step::Call(|harness| assert!(!harness.cameras.borrow().is_open(PORT))),
    ]);

    let output = &transcript.output;
    assert!(output.contains(REMOVED), "{output}");
    assert_eq!(output.matches("[REMOVED] Fujifilm X-T5\n").count(), 1);
    assert_eq!(harness.memory.content(&imported(2)), None);
}

/// spec: CLI-014.11
#[test]
fn a_camera_with_nothing_to_import_is_reported() {
    let harness = Harness::new();
    harness.remember(Some(PICTURES), Some(PRESET_1), &[]);
    harness
        .camera_storage
        .add_folder(Path::new("/store_00010001/DCIM"));
    connect(&harness);

    let transcript = harness.run(vec![Step::Tick]);

    assert!(
        transcript.output.contains(&format!(
            "Reading Fujifilm X-T5…\nFujifilm X-T5 has no photos or videos to import.\n{WAITING}"
        )),
        "{}",
        transcript.output
    );
}
