// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The devices service of Framehive: the [`DeviceSource`] port, which lists the
//! devices offered for import and ejects them, and the [`DeviceWatch`], which turns
//! successive lists into changes (DEVICES-001, ADR 0023).
//!
//! The service never reads mounts itself; `adapter-mounts` implements the port.
//!
//! ```
//! use devices::DeviceWatch;
//! use devices::testing::MemoryDeviceSource;
//!
//! let source = MemoryDeviceSource::new();
//! let mut watch = DeviceWatch::new();
//! source.insert_card("/run/media/anna/EOS_DIGITAL");
//!
//! let changes = watch.poll(&source)?;
//! assert_eq!(changes.added[0].name(), "EOS_DIGITAL");
//! assert!(watch.poll(&source)?.is_empty());
//! # Ok::<(), devices::ListError>(())
//! ```

#![forbid(unsafe_code)]

mod device;
mod error;
mod port;
pub mod testing;
mod watch;

pub use device::Device;
pub use error::{EjectError, ListError};
pub use port::DeviceSource;
pub use watch::{DeviceChanges, DeviceWatch};
