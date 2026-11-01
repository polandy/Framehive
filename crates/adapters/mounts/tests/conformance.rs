// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: [`adapter_mounts::MountsDeviceSource`] passes the conformance
//! suite of the `devices::DeviceSource` port, the Linux way with a fake `mountinfo`
//! file and the macOS way with a fake `Volumes` folder, on every platform (CLAUDE.md,
//! "Adding things"). Stand-ins for `udisksctl` and `diskutil` eject.

mod support;

use devices::testing::check_device_source;
use support::{LinuxSystem, MacOsSystem};

/// spec: DEVICES-001.1
/// spec: DEVICES-001.2
/// spec: DEVICES-001.3
/// spec: DEVICES-001.7
/// spec: DEVICES-001.11
/// spec: DEVICES-001.13
/// spec: DEVICES-001.14
#[test]
fn conforms_to_the_device_source_port_on_linux() {
    let failures = check_device_source(&mut LinuxSystem::new);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// spec: DEVICES-001.1
/// spec: DEVICES-001.2
/// spec: DEVICES-001.3
/// spec: DEVICES-001.7
/// spec: DEVICES-001.11
/// spec: DEVICES-001.13
/// spec: DEVICES-001.14
#[test]
fn conforms_to_the_device_source_port_on_macos() {
    let failures = check_device_source(&mut MacOsSystem::new);
    assert!(failures.is_empty(), "{failures:#?}");
}
