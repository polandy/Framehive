// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The device watch turns successive lists into changes (DEVICES-001.8–10).

use devices::testing::MemoryDeviceSource;
use devices::{Device, DeviceChanges, DevicePoll, DeviceWatch, PollError};
use files::testing::{MemoryCameras, MemoryFileSystem};

const CARD_A: &str = "/run/media/anna/CARD_A";
const CARD_B: &str = "/run/media/anna/CARD_B";
const CARD_C: &str = "/run/media/anna/CARD_C";

/// Polls without cameras and expects that nothing fails.
fn poll(watch: &mut DeviceWatch, cards: &MemoryDeviceSource) -> DeviceChanges {
    let result = watch.poll(cards, &MemoryCameras::new());
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    result.changes
}

fn cameras(connected: &[(&str, &str)]) -> MemoryCameras {
    let file_system = MemoryFileSystem::new();
    connected
        .iter()
        .fold(MemoryCameras::new(), |cameras, (model, port)| {
            cameras.with_camera(model, port, &file_system)
        })
}

fn camera_changes(removed: &[(&str, &str)], added: &[(&str, &str)]) -> DeviceChanges {
    let devices = |list: &[(&str, &str)]| {
        list.iter()
            .map(|(model, port)| Device::camera(*model, *port))
            .collect()
    };
    DeviceChanges {
        removed: devices(removed),
        added: devices(added),
    }
}

fn changes(removed: &[&str], added: &[&str]) -> DeviceChanges {
    DeviceChanges {
        removed: removed.iter().map(Device::at).collect(),
        added: added.iter().map(Device::at).collect(),
    }
}

/// spec: DEVICES-001.8
#[test]
fn the_first_poll_reports_every_device_as_added() {
    let source = MemoryDeviceSource::new();
    source.insert_card(CARD_B);
    source.insert_card(CARD_A);
    let mut watch = DeviceWatch::new();

    let reported = poll(&mut watch, &source);

    assert_eq!(reported, changes(&[], &[CARD_A, CARD_B]));
    assert_eq!(watch.devices(), [Device::at(CARD_A), Device::at(CARD_B)]);
}

/// spec: DEVICES-001.9
#[test]
fn a_later_poll_reports_removed_and_added_devices_only() {
    let source = MemoryDeviceSource::new();
    source.insert_card(CARD_A);
    source.insert_card(CARD_B);
    let mut watch = DeviceWatch::new();
    poll(&mut watch, &source);
    source.remove(CARD_A.as_ref());
    source.insert_card(CARD_C);

    assert_eq!(poll(&mut watch, &source), changes(&[CARD_A], &[CARD_C]));
}

/// spec: DEVICES-001.9
#[test]
fn a_poll_with_an_unchanged_list_reports_nothing() {
    let source = MemoryDeviceSource::new();
    source.insert_card(CARD_A);
    let mut watch = DeviceWatch::new();
    poll(&mut watch, &source);

    let reported = poll(&mut watch, &source);

    assert!(reported.is_empty(), "{reported:?}");
}

#[test]
fn removed_and_added_devices_are_each_sorted() {
    let source = MemoryDeviceSource::new();
    source.insert_card(CARD_C);
    source.insert_card(CARD_B);
    let mut watch = DeviceWatch::new();
    poll(&mut watch, &source);
    source.remove(CARD_C.as_ref());
    source.remove(CARD_B.as_ref());
    source.insert_card("/media/z/Z");
    source.insert_card(CARD_A);

    let reported = poll(&mut watch, &source);

    assert_eq!(
        reported,
        changes(&[CARD_B, CARD_C], &["/media/z/Z", CARD_A])
    );
}

/// spec: DEVICES-001.10
#[test]
fn a_failed_poll_returns_the_error_and_keeps_the_last_list() {
    let source = MemoryDeviceSource::new();
    source.insert_card(CARD_A);
    let mut watch = DeviceWatch::new();
    poll(&mut watch, &source);
    source.remove(CARD_A.as_ref());
    source.fail_listing(Some("/proc/self/mountinfo"));

    let result = watch.poll(&source, &MemoryCameras::new());

    assert!(result.changes.is_empty());
    assert!(
        matches!(&result.errors[..], [PollError::Cards(error)] if error.what() == "/proc/self/mountinfo")
    );
    assert_eq!(watch.devices(), [Device::at(CARD_A)]);

    source.fail_listing(None);
    assert_eq!(poll(&mut watch, &source), changes(&[CARD_A], &[]));
}

const X_T5: (&str, &str) = ("Fujifilm X-T5", "usb:002,007");

/// spec: DEVICES-001.17
#[test]
fn a_camera_is_reported_as_added_and_removed_after_the_cards() {
    let cards = MemoryDeviceSource::new();
    cards.insert_card(CARD_A);
    let mut watch = DeviceWatch::new();

    let first = watch.poll(&cards, &cameras(&[X_T5]));

    assert!(first.errors.is_empty());
    assert_eq!(
        first.changes.added,
        [Device::at(CARD_A), Device::camera(X_T5.0, X_T5.1)]
    );
    assert_eq!(watch.devices(), first.changes.added);

    let unplugged = watch.poll(&cards, &cameras(&[]));
    assert_eq!(unplugged.changes, camera_changes(&[X_T5], &[]));
    assert_eq!(watch.devices(), [Device::at(CARD_A)]);
}

/// spec: DEVICES-001.18
#[test]
fn a_camera_plugged_in_again_at_a_new_port_is_removed_and_added() {
    let cards = MemoryDeviceSource::new();
    let mut watch = DeviceWatch::new();
    watch.poll(&cards, &cameras(&[X_T5]));
    let replugged = ("Fujifilm X-T5", "usb:002,008");

    let result = watch.poll(&cards, &cameras(&[replugged]));

    assert_eq!(result.changes, camera_changes(&[X_T5], &[replugged]));
}

/// spec: DEVICES-001.19
#[test]
fn failing_cameras_keep_the_last_cameras_and_card_changes_are_still_reported() {
    let cards = MemoryDeviceSource::new();
    cards.insert_card(CARD_A);
    let mut watch = DeviceWatch::new();
    watch.poll(&cards, &cameras(&[X_T5]));
    cards.remove(CARD_A.as_ref());
    cards.insert_card(CARD_B);
    let failing = MemoryCameras::new().with_list_failure("gphoto2 is gone");

    let result = watch.poll(&cards, &failing);

    assert_eq!(result.changes, changes(&[CARD_A], &[CARD_B]));
    assert!(matches!(&result.errors[..], [PollError::Cameras(_)]));
    assert_eq!(
        watch.devices(),
        [Device::at(CARD_B), Device::camera(X_T5.0, X_T5.1)]
    );
}

/// spec: DEVICES-001.20
#[test]
fn failing_cards_keep_the_last_cards_and_camera_changes_are_still_reported() {
    let cards = MemoryDeviceSource::new();
    cards.insert_card(CARD_A);
    let mut watch = DeviceWatch::new();
    watch.poll(&cards, &cameras(&[X_T5]));
    cards.fail_listing(Some("/proc/self/mountinfo"));

    let result: DevicePoll = watch.poll(&cards, &cameras(&[]));

    assert_eq!(result.changes, camera_changes(&[X_T5], &[]));
    assert!(matches!(&result.errors[..], [PollError::Cards(_)]));
    assert_eq!(watch.devices(), [Device::at(CARD_A)]);
}
