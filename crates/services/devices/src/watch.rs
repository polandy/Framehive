// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The [`DeviceWatch`], which turns successive lists into changes.

use crate::{Device, DeviceSource, ListError};

/// What changed between two successful polls of a [`DeviceWatch`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceChanges {
    /// The devices that were removed, sorted by mount point. Reported first.
    pub removed: Vec<Device>,
    /// The devices that were added, sorted by mount point.
    pub added: Vec<Device>,
}

impl DeviceChanges {
    /// Whether nothing changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.removed.is_empty() && self.added.is_empty()
    }
}

/// Compares the list of each poll with the list of the last successful poll
/// (DEVICES-001.8–10). It never waits; the caller decides when to poll (ADR 0023 §3).
#[derive(Debug, Clone, Default)]
pub struct DeviceWatch {
    last: Vec<Device>,
}

impl DeviceWatch {
    /// A watch that has not polled yet; its last list is empty.
    #[must_use]
    pub fn new() -> Self {
        DeviceWatch::default()
    }

    /// The devices of the last successful poll, sorted by mount point.
    #[must_use]
    pub fn devices(&self) -> &[Device] {
        &self.last
    }

    /// Lists the devices of `source` and reports what changed since the last
    /// successful poll.
    ///
    /// # Errors
    ///
    /// Returns the source's [`ListError`]; the watch then keeps its last list.
    pub fn poll(&mut self, source: &dyn DeviceSource) -> Result<DeviceChanges, ListError> {
        let mut current = source.list()?;
        current.sort();
        current.dedup();
        let changes = DeviceChanges {
            removed: missing_from(&self.last, &current),
            added: missing_from(&current, &self.last),
        };
        self.last = current;
        Ok(changes)
    }
}

/// The devices of `devices` that `other` does not hold, in the order of `devices`.
fn missing_from(devices: &[Device], other: &[Device]) -> Vec<Device> {
    devices
        .iter()
        .filter(|device| other.binary_search(device).is_err())
        .cloned()
        .collect()
}
