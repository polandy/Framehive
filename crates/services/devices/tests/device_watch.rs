// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The device watch turns successive lists into changes (DEVICES-001.8–10).

use devices::testing::MemoryDeviceSource;
use devices::{Device, DeviceChanges, DeviceWatch};

const CARD_A: &str = "/run/media/anna/CARD_A";
const CARD_B: &str = "/run/media/anna/CARD_B";
const CARD_C: &str = "/run/media/anna/CARD_C";

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

    let reported = watch.poll(&source).unwrap();

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
    watch.poll(&source).unwrap();
    source.remove(CARD_A.as_ref());
    source.insert_card(CARD_C);

    assert_eq!(watch.poll(&source).unwrap(), changes(&[CARD_A], &[CARD_C]));
}

/// spec: DEVICES-001.9
#[test]
fn a_poll_with_an_unchanged_list_reports_nothing() {
    let source = MemoryDeviceSource::new();
    source.insert_card(CARD_A);
    let mut watch = DeviceWatch::new();
    watch.poll(&source).unwrap();

    let reported = watch.poll(&source).unwrap();

    assert!(reported.is_empty(), "{reported:?}");
}

#[test]
fn removed_and_added_devices_are_each_sorted() {
    let source = MemoryDeviceSource::new();
    source.insert_card(CARD_C);
    source.insert_card(CARD_B);
    let mut watch = DeviceWatch::new();
    watch.poll(&source).unwrap();
    source.remove(CARD_C.as_ref());
    source.remove(CARD_B.as_ref());
    source.insert_card("/media/z/Z");
    source.insert_card(CARD_A);

    let reported = watch.poll(&source).unwrap();

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
    watch.poll(&source).unwrap();
    source.remove(CARD_A.as_ref());
    source.fail_listing(Some("/proc/self/mountinfo"));

    let error = watch.poll(&source).unwrap_err();

    assert_eq!(error.what(), "/proc/self/mountinfo");
    assert_eq!(watch.devices(), [Device::at(CARD_A)]);

    source.fail_listing(None);
    assert_eq!(watch.poll(&source).unwrap(), changes(&[CARD_A], &[]));
}
