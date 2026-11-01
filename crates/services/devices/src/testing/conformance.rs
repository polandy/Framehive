// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`DeviceSource`] port (DEVICES-001).

use std::path::{Path, PathBuf};

use crate::{Device, DeviceSource, EjectError};

/// What a mounted file system holds at its top level under the name `DCIM`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DcimFolder {
    /// A folder `DCIM`: the file system is a card (DCF).
    Folder,
    /// Nothing named `DCIM`.
    Missing,
    /// A file named `DCIM`.
    File,
}

/// A device source and the environment around it, for [`check_device_source`].
///
/// Each check starts from a fresh one without mounts.
pub trait DeviceSourceUnderTest {
    /// The implementation under test.
    fn source(&self) -> &dyn DeviceSource;

    /// Mounts a file system named `name` at a location the source considers, with
    /// `DCIM` as given, and returns its mount point.
    fn insert(&mut self, name: &str, dcim: DcimFolder) -> PathBuf;

    /// Unmounts the file system at `mount_point` without the source's help, as
    /// pulling a card does.
    fn remove(&mut self, mount_point: &Path);

    /// Makes listing fail from now on and returns the text that the error must name,
    /// such as the path of the mount table.
    fn break_listing(&mut self) -> String;

    /// Makes every eject fail from now on, with the tool message `message`.
    fn fail_ejects(&mut self, message: &str);
}

/// Runs the conformance suite of the [`DeviceSource`] port and returns one message
/// per failed check, or nothing if the implementation conforms.
///
/// `fresh` makes a new environment without mounts for each check.
#[must_use]
pub fn check_device_source<T: DeviceSourceUnderTest>(fresh: &mut dyn FnMut() -> T) -> Vec<String> {
    let mut failures = Vec::new();
    check_card_listed(&mut fresh(), &mut failures);
    check_without_dcim(&mut fresh(), &mut failures);
    check_sorted(&mut fresh(), &mut failures);
    check_list_error(&mut fresh(), &mut failures);
    check_eject(&mut fresh(), &mut failures);
    check_failed_eject(&mut fresh(), &mut failures);
    check_eject_not_present(&mut fresh(), &mut failures);
    failures
}

fn listed(
    under_test: &dyn DeviceSourceUnderTest,
    check: &str,
    failures: &mut Vec<String>,
) -> Vec<Device> {
    under_test.source().list().unwrap_or_else(|error| {
        failures.push(format!("{check}: listing fails: {error}"));
        Vec::new()
    })
}

fn check_card_listed(under_test: &mut dyn DeviceSourceUnderTest, failures: &mut Vec<String>) {
    let mount_point = under_test.insert("EOS_DIGITAL", DcimFolder::Folder);
    let devices = listed(under_test, "card_listed", failures);
    if devices != [Device::at(&mount_point)] {
        failures.push(format!(
            "card_listed: a card at {} is listed as {devices:?}",
            mount_point.display()
        ));
    }
    if let Some(device) = devices.first()
        && device.name() != "EOS_DIGITAL"
    {
        failures.push(format!(
            "card_listed: the card is named {}, expected EOS_DIGITAL",
            device.name()
        ));
    }
}

fn check_without_dcim(under_test: &mut dyn DeviceSourceUnderTest, failures: &mut Vec<String>) {
    under_test.insert("BACKUP", DcimFolder::Missing);
    under_test.insert("STICK", DcimFolder::File);
    let devices = listed(under_test, "without_dcim", failures);
    if !devices.is_empty() {
        failures.push(format!(
            "without_dcim: mounts without a DCIM folder are listed as {devices:?}"
        ));
    }
}

fn check_sorted(under_test: &mut dyn DeviceSourceUnderTest, failures: &mut Vec<String>) {
    let mut expected: Vec<Device> = ["b", "A B", "a", "Z", "A"]
        .into_iter()
        .map(|name| Device::at(under_test.insert(name, DcimFolder::Folder)))
        .collect();
    expected.sort_by(|left, right| {
        let bytes = |device: &Device| device.mount_point().as_os_str().as_encoded_bytes().to_vec();
        bytes(left).cmp(&bytes(right))
    });
    let devices = listed(under_test, "sorted", failures);
    if devices != expected {
        failures.push(format!("sorted: listed {devices:?}, expected {expected:?}"));
    }
}

fn check_list_error(under_test: &mut dyn DeviceSourceUnderTest, failures: &mut Vec<String>) {
    under_test.insert("EOS_DIGITAL", DcimFolder::Folder);
    let what = under_test.break_listing();
    match under_test.source().list() {
        Ok(devices) => failures.push(format!(
            "list_error: listing succeeds with {devices:?} although {what} cannot be read"
        )),
        Err(error) if !error.to_string().contains(&what) => failures.push(format!(
            "list_error: the error \"{error}\" does not name {what}"
        )),
        Err(_) => {}
    }
}

fn check_eject(under_test: &mut dyn DeviceSourceUnderTest, failures: &mut Vec<String>) {
    let card = Device::at(under_test.insert("EOS_DIGITAL", DcimFolder::Folder));
    let other = Device::at(under_test.insert("Untitled", DcimFolder::Folder));
    if let Err(error) = under_test.source().eject(&card) {
        failures.push(format!("eject: ejecting a listed card fails: {error}"));
    }
    let devices = listed(under_test, "eject", failures);
    if devices != [other] {
        failures.push(format!(
            "eject: after ejecting {card} the source lists {devices:?}"
        ));
    }
}

fn check_failed_eject(under_test: &mut dyn DeviceSourceUnderTest, failures: &mut Vec<String>) {
    let card = Device::at(under_test.insert("EOS_DIGITAL", DcimFolder::Folder));
    let message = "target is busy";
    under_test.fail_ejects(message);
    match under_test.source().eject(&card) {
        Ok(()) => {
            failures.push("failed_eject: the eject succeeds although the tool fails".to_owned());
        }
        Err(error @ EjectError::Failed { .. }) => {
            let text = error.to_string();
            let mount_point = card.mount_point().display().to_string();
            if !text.contains(&mount_point) || !text.contains(message) {
                failures.push(format!(
                    "failed_eject: the error \"{text}\" does not name {mount_point} and \"{message}\""
                ));
            }
        }
        Err(error) => failures.push(format!(
            "failed_eject: expected a failed eject, got \"{error}\""
        )),
    }
    let devices = listed(under_test, "failed_eject", failures);
    if devices != [card] {
        failures.push(format!(
            "failed_eject: after a failed eject the source lists {devices:?}"
        ));
    }
}

fn check_eject_not_present(under_test: &mut dyn DeviceSourceUnderTest, failures: &mut Vec<String>) {
    let mount_point = under_test.insert("EOS_DIGITAL", DcimFolder::Folder);
    under_test.remove(&mount_point);
    let pulled = Device::at(mount_point);
    match under_test.source().eject(&pulled) {
        Err(EjectError::NotPresent { device }) if device == pulled => {}
        Ok(()) => failures.push(format!(
            "eject_not_present: ejecting the pulled {pulled} succeeds"
        )),
        Err(error) => failures.push(format!(
            "eject_not_present: expected \"not present\" for {pulled}, got \"{error}\""
        )),
    }
}
