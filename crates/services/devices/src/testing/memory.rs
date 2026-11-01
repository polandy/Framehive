// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A [`DeviceSource`] in memory.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::testing::DcimFolder;
use crate::{Device, DeviceSource, EjectError, ListError};

/// A [`DeviceSource`] in memory: tests insert and remove mounts, and make listing or
/// ejecting fail.
///
/// Like a real source, it lists only mounts with a `DCIM` folder.
///
/// ```
/// use devices::DeviceSource;
/// use devices::testing::MemoryDeviceSource;
///
/// let source = MemoryDeviceSource::new();
/// let card = source.insert_card("/media/anna/EOS_DIGITAL");
/// source.eject(&card)?;
/// assert!(source.list()?.is_empty());
/// # Ok::<(), devices::EjectError>(())
/// ```
#[derive(Debug, Default)]
pub struct MemoryDeviceSource {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    mounts: Vec<(PathBuf, DcimFolder)>,
    list_failure: Option<String>,
    eject_failure: Option<String>,
}

impl MemoryDeviceSource {
    /// A source without mounts.
    #[must_use]
    pub fn new() -> Self {
        MemoryDeviceSource::default()
    }

    /// Mounts a card with a `DCIM` folder at `mount_point` and returns its device.
    pub fn insert_card(&self, mount_point: impl Into<PathBuf>) -> Device {
        let mount_point = mount_point.into();
        self.insert(&mount_point, DcimFolder::Folder);
        Device::at(mount_point)
    }

    /// Mounts a file system at `mount_point` whose `DCIM` is as given; only a
    /// [`DcimFolder::Folder`] makes it a device.
    pub fn insert(&self, mount_point: &Path, dcim: DcimFolder) {
        let mut state = self.lock();
        state.mounts.retain(|(mounted, _)| mounted != mount_point);
        state.mounts.push((mount_point.to_path_buf(), dcim));
    }

    /// Unmounts whatever is mounted at `mount_point`, as pulling a card does.
    pub fn remove(&self, mount_point: &Path) {
        self.lock()
            .mounts
            .retain(|(mounted, _)| mounted != mount_point);
    }

    /// Makes every listing fail with an error naming `what`, or, with `None`,
    /// succeed again.
    pub fn fail_listing(&self, what: Option<&str>) {
        self.lock().list_failure = what.map(str::to_owned);
    }

    /// Makes every eject fail with the tool message `message`, or, with `None`,
    /// succeed again.
    pub fn fail_ejects(&self, message: Option<&str>) {
        self.lock().eject_failure = message.map(str::to_owned);
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // A test that panicked while holding the lock leaves consistent state behind.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl DeviceSource for MemoryDeviceSource {
    fn list(&self) -> Result<Vec<Device>, ListError> {
        let state = self.lock();
        if let Some(what) = &state.list_failure {
            return Err(ListError::new(
                what.clone(),
                io::Error::other("listing fails in this test"),
            ));
        }
        let mut devices: Vec<Device> = state
            .mounts
            .iter()
            .filter(|(_, dcim)| *dcim == DcimFolder::Folder)
            .map(|(mount_point, _)| Device::at(mount_point))
            .collect();
        devices.sort();
        Ok(devices)
    }

    fn eject(&self, device: &Device) -> Result<(), EjectError> {
        if !self.list()?.contains(device) {
            return Err(EjectError::NotPresent {
                device: device.clone(),
            });
        }
        let mut state = self.lock();
        if let Some(message) = &state.eject_failure {
            return Err(EjectError::Failed {
                device: device.clone(),
                message: message.clone(),
            });
        }
        state
            .mounts
            .retain(|(mounted, _)| mounted != device.mount_point());
        Ok(())
    }
}
