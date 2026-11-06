// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The [`DeviceWatch`], which turns successive lists into changes.

use files::{CameraError, Cameras};
use thiserror::Error;

use crate::{Device, DeviceSource, ListError};

/// What changed between two successful polls of a [`DeviceWatch`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceChanges {
    /// The devices that were removed, sorted (cards, then cameras). Reported first.
    pub removed: Vec<Device>,
    /// The devices that were added, sorted (cards, then cameras).
    pub added: Vec<Device>,
}

impl DeviceChanges {
    /// Whether nothing changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.removed.is_empty() && self.added.is_empty()
    }
}

/// One kind of device could not be listed during a poll (DEVICES-001.19).
#[derive(Debug, Error)]
pub enum PollError {
    /// The cards could not be listed.
    #[error(transparent)]
    Cards(ListError),
    /// The cameras could not be listed.
    #[error(transparent)]
    Cameras(CameraError),
}

/// The result of one poll: the changes, and one error for each kind of device that
/// could not be listed.
#[derive(Debug, Default)]
pub struct DevicePoll {
    /// What changed; a kind that failed to list contributes nothing.
    pub changes: DeviceChanges,
    /// The kinds that could not be listed; the watch kept their last devices.
    pub errors: Vec<PollError>,
}

/// Compares the list of each poll with the list of the last poll
/// (DEVICES-001.8–10, 17–20). It never waits; the caller decides when to poll
/// (ADR 0023 §3).
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

    /// The devices of the last poll, sorted: cards, then cameras.
    #[must_use]
    pub fn devices(&self) -> &[Device] {
        &self.last
    }

    /// Lists the cards of `cards` and the cameras of `cameras` independently and
    /// reports what changed since the last poll. A kind that cannot be listed keeps
    /// its last devices and adds an error to [`DevicePoll::errors`]; the other kind
    /// is still reported.
    pub fn poll(&mut self, cards: &dyn DeviceSource, cameras: &dyn Cameras) -> DevicePoll {
        let mut errors = Vec::new();
        let mut current: Vec<Device> = Vec::new();
        match cards.list() {
            Ok(listed) => current.extend(listed),
            Err(error) => {
                current.extend(
                    self.last
                        .iter()
                        .filter(|device| !device.is_camera())
                        .cloned(),
                );
                errors.push(PollError::Cards(error));
            }
        }
        match cameras.list() {
            Ok(listed) => current.extend(
                listed
                    .into_iter()
                    .map(|camera| Device::camera(camera.model, camera.port)),
            ),
            Err(error) => {
                current.extend(
                    self.last
                        .iter()
                        .filter(|device| device.is_camera())
                        .cloned(),
                );
                errors.push(PollError::Cameras(error));
            }
        }
        current.sort();
        current.dedup();
        let changes = DeviceChanges {
            removed: missing_from(&self.last, &current),
            added: missing_from(&current, &self.last),
        };
        self.last = current;
        DevicePoll { changes, errors }
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
