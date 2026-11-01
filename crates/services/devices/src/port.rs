// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The [`DeviceSource`] port.

use crate::{Device, EjectError, ListError};

/// Port through which Framehive finds the devices offered for import and ejects them
/// (DEVICES-001, ADR 0023 §1); implemented by `adapter-mounts`.
///
/// Every implementation passes [`testing::check_device_source`](crate::testing::check_device_source).
pub trait DeviceSource {
    /// Lists the devices that are present, sorted by mount point in byte order.
    ///
    /// # Errors
    ///
    /// Returns a [`ListError`] naming what could not be read.
    fn list(&self) -> Result<Vec<Device>, ListError>;

    /// Ejects `device`, so that it can be pulled safely; afterwards it is no longer
    /// listed.
    ///
    /// # Errors
    ///
    /// Returns [`EjectError::NotPresent`] if `device` is not listed, and
    /// [`EjectError::Failed`] if ejecting failed; the device then stays listed.
    fn eject(&self, device: &Device) -> Result<(), EjectError>;
}
