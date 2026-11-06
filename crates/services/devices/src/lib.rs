// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The devices service of Framehive: the [`DeviceSource`] port, which lists the
//! devices offered for import and ejects them, and the [`DeviceWatch`], which turns
//! successive lists into changes (DEVICES-001, ADR 0023).
//!
//! The service never reads mounts itself; `adapter-mounts` implements the port. A
//! device is a card or a camera; the watch lists cameras through the `Cameras` port of
//! the files service.
//!
//! ```
//! use devices::DeviceWatch;
//! use devices::testing::MemoryDeviceSource;
//! use files::testing::MemoryCameras;
//!
//! let cards = MemoryDeviceSource::new();
//! let cameras = MemoryCameras::new();
//! let mut watch = DeviceWatch::new();
//! cards.insert_card("/run/media/anna/EOS_DIGITAL");
//!
//! let poll = watch.poll(&cards, &cameras);
//! assert_eq!(poll.changes.added[0].name(), "EOS_DIGITAL");
//! assert!(poll.errors.is_empty());
//! assert!(watch.poll(&cards, &cameras).changes.is_empty());
//! ```

#![forbid(unsafe_code)]

mod device;
mod error;
mod port;
pub mod testing;
mod watch;

pub use device::{Device, DeviceKind};
pub use error::{EjectError, ListError};
pub use port::DeviceSource;
pub use watch::{DeviceChanges, DevicePoll, DeviceWatch, PollError};
