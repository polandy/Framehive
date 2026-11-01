// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Errors of the [`DeviceSource`](crate::DeviceSource) port.

use std::io;

use thiserror::Error;

use crate::Device;

/// The devices could not be listed (DEVICES-001.7).
#[derive(Debug, Error)]
#[error("cannot read {what}: {source}")]
pub struct ListError {
    what: String,
    source: io::Error,
}

impl ListError {
    /// Listing failed because `what`, such as `/proc/self/mountinfo`, could not be
    /// read.
    #[must_use]
    pub fn new(what: impl Into<String>, source: io::Error) -> Self {
        ListError {
            what: what.into(),
            source,
        }
    }

    /// What could not be read.
    #[must_use]
    pub fn what(&self) -> &str {
        &self.what
    }
}

/// A device could not be ejected; it stays listed.
#[derive(Debug, Error)]
pub enum EjectError {
    /// The device source does not list the device, for example because it was
    /// pulled (DEVICES-001.14).
    #[error("cannot eject {device}: the device is not present")]
    NotPresent {
        /// The device that was to be ejected.
        device: Device,
    },
    /// The eject tool could not be started or reported a failure (DEVICES-001.13).
    #[error("cannot eject {device}: {message}")]
    Failed {
        /// The device that was to be ejected.
        device: Device,
        /// The tool's message or exit status.
        message: String,
    },
    /// The devices could not be listed to find the device.
    #[error(transparent)]
    List(#[from] ListError),
}
