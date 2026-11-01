// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The memory device source behaves as DEVICES-001 asks of every device source, and
//! passes the conformance suite of the port.

use std::path::{Path, PathBuf};

use devices::testing::{
    DcimFolder, DeviceSourceUnderTest, MemoryDeviceSource, check_device_source,
};
use devices::{Device, DeviceSource, EjectError};

const CARD: &str = "/run/media/anna/EOS_DIGITAL";

/// spec: DEVICES-001.1
#[test]
fn a_card_is_listed_with_its_name_and_mount_point() {
    let source = MemoryDeviceSource::new();
    source.insert_card(CARD);

    let devices = source.list().unwrap();

    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].name(), "EOS_DIGITAL");
    assert_eq!(devices[0].mount_point(), Path::new(CARD));
}

/// spec: DEVICES-001.2
#[test]
fn a_mount_without_a_dcim_folder_is_not_listed() {
    let source = MemoryDeviceSource::new();
    source.insert(Path::new("/run/media/anna/BACKUP"), DcimFolder::Missing);
    source.insert(Path::new("/run/media/anna/STICK"), DcimFolder::File);

    assert_eq!(source.list().unwrap(), []);
}

/// spec: DEVICES-001.3
#[test]
fn cards_are_listed_sorted_by_mount_point() {
    let source = MemoryDeviceSource::new();
    for mount_point in ["/media/b/Z", "/media/a/Y", "/media/a b/X"] {
        source.insert_card(mount_point);
    }

    let mount_points: Vec<PathBuf> = source
        .list()
        .unwrap()
        .iter()
        .map(|device| device.mount_point().to_path_buf())
        .collect();

    assert_eq!(
        mount_points,
        ["/media/a b/X", "/media/a/Y", "/media/b/Z"].map(PathBuf::from)
    );
}

/// spec: DEVICES-001.7
#[test]
fn a_failed_listing_names_what_could_not_be_read() {
    let source = MemoryDeviceSource::new();
    source.fail_listing(Some("/proc/self/mountinfo"));

    let error = source.list().unwrap_err();

    assert!(
        error
            .to_string()
            .starts_with("cannot read /proc/self/mountinfo: "),
        "{error}"
    );
}

/// spec: DEVICES-001.11
#[test]
fn an_ejected_card_is_no_longer_listed() {
    let source = MemoryDeviceSource::new();
    let card = source.insert_card(CARD);

    source.eject(&card).unwrap();

    assert_eq!(source.list().unwrap(), []);
}

/// spec: DEVICES-001.13
#[test]
fn a_failed_eject_names_the_device_and_the_tool_message_and_keeps_it_listed() {
    let source = MemoryDeviceSource::new();
    let card = source.insert_card(CARD);
    source.fail_ejects(Some("target is busy"));

    let error = source.eject(&card).unwrap_err();

    assert_eq!(
        error.to_string(),
        "cannot eject EOS_DIGITAL (/run/media/anna/EOS_DIGITAL): target is busy"
    );
    assert_eq!(source.list().unwrap(), [card]);
}

/// spec: DEVICES-001.14
#[test]
fn ejecting_a_pulled_card_says_it_is_not_present() {
    let source = MemoryDeviceSource::new();
    let card = source.insert_card(CARD);
    source.remove(Path::new(CARD));

    let error = source.eject(&card).unwrap_err();

    assert!(matches!(&error, EjectError::NotPresent { device } if *device == card));
    assert_eq!(
        error.to_string(),
        "cannot eject EOS_DIGITAL (/run/media/anna/EOS_DIGITAL): the device is not present"
    );
}

/// The memory source in a fresh environment for each check of the suite.
struct MemoryUnderTest {
    source: MemoryDeviceSource,
}

impl DeviceSourceUnderTest for MemoryUnderTest {
    fn source(&self) -> &dyn DeviceSource {
        &self.source
    }

    fn insert(&mut self, name: &str, dcim: DcimFolder) -> PathBuf {
        let mount_point = Path::new("/run/media/anna").join(name);
        self.source.insert(&mount_point, dcim);
        mount_point
    }

    fn remove(&mut self, mount_point: &Path) {
        self.source.remove(mount_point);
    }

    fn break_listing(&mut self) -> String {
        self.source.fail_listing(Some("the mount table"));
        "the mount table".to_owned()
    }

    fn fail_ejects(&mut self, message: &str) {
        self.source.fail_ejects(Some(message));
    }
}

/// spec: DEVICES-001.1
/// spec: DEVICES-001.2
/// spec: DEVICES-001.3
/// spec: DEVICES-001.7
/// spec: DEVICES-001.11
/// spec: DEVICES-001.13
/// spec: DEVICES-001.14
#[test]
fn memory_device_source_conforms() {
    let failures = check_device_source(&mut || MemoryUnderTest {
        source: MemoryDeviceSource::new(),
    });

    assert_eq!(failures, Vec::<String>::new());
}

#[test]
fn a_device_is_displayed_with_its_name_and_mount_point() {
    assert_eq!(
        Device::at(CARD).to_string(),
        "EOS_DIGITAL (/run/media/anna/EOS_DIGITAL)"
    );
}
