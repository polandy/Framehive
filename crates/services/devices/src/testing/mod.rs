// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Test doubles and the conformance suite of the [`DeviceSource`](crate::DeviceSource)
//! port (ADR 0023 §1).
//!
//! `adapter-mounts` runs [`check_device_source`] against a fake mount table.

mod conformance;
mod memory;

pub use conformance::{DcimFolder, DeviceSourceUnderTest, check_device_source};
pub use memory::MemoryDeviceSource;
