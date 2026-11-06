// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A camera that goes during the guided import (CLI-014): disconnected, stopping to
//! answer, or with one failed file.

use std::time::Duration;

use files::testing::{Fault, MemoryFileSystem};

use super::cameras::{
    MODEL, OTHER_PORT, PORT, REMOVED, add_photos, change_cameras, connect, disconnect, imported,
    photo, with_photos,
};
use super::support::{Harness, Step};

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

/// spec: CLI-014.10
#[test]
fn a_single_failed_file_does_not_stop_the_import() {
    let harness = with_photos(3);
    harness.time_per_copy.set(Duration::from_secs(1));
    harness
        .camera_storage
        .inject_fault(&photo(2), Fault::ReadFails { after_bytes: 10 });
    connect(&harness);

    let transcript = harness.run(vec![Step::Line(""), Step::Tick, Step::Line("n")]);

    let output = &transcript.output;
    assert!(!output.contains("[REMOVED]"), "{output}");
    assert!(
        !output.contains("disconnected during the import"),
        "{output}"
    );
    assert_eq!(
        harness.memory.content(&imported(3)),
        Some(vec![3; 1000]),
        "{output}"
    );
}
