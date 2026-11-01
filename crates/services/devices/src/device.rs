// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A device as it is offered for import.

use std::cmp::Ordering;
use std::fmt;
use std::path::{Path, PathBuf};

/// A removable storage medium, such as a memory card, as it is offered for import
/// (glossary). Its mount point identifies it while it is mounted.
///
/// Devices are ordered by their mount points in byte order of the path
/// (DEVICES-001.3), which is not the component order of [`Path`].
///
/// ```
/// use devices::Device;
///
/// let device = Device::at("/run/media/anna/EOS_DIGITAL");
/// assert_eq!(device.name(), "EOS_DIGITAL");
/// assert_eq!(device.to_string(), "EOS_DIGITAL (/run/media/anna/EOS_DIGITAL)");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Device {
    name: String,
    mount_point: PathBuf,
}

impl Device {
    /// The device mounted at `mount_point`; its name is the last component of the
    /// mount point, or the whole mount point if it has no last component, such as `/`.
    #[must_use]
    pub fn at(mount_point: impl Into<PathBuf>) -> Self {
        let mount_point = mount_point.into();
        let name = mount_point
            .file_name()
            .unwrap_or(mount_point.as_os_str())
            .to_string_lossy()
            .into_owned();
        Device { name, mount_point }
    }

    /// The name shown to the user, such as `EOS_DIGITAL`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where the device is mounted.
    #[must_use]
    pub fn mount_point(&self) -> &Path {
        &self.mount_point
    }
}

impl Ord for Device {
    fn cmp(&self, other: &Self) -> Ordering {
        mount_point_bytes(self).cmp(mount_point_bytes(other))
    }
}

impl PartialOrd for Device {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Device {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({})", self.name, self.mount_point.display())
    }
}

fn mount_point_bytes(device: &Device) -> &[u8] {
    device.mount_point.as_os_str().as_encoded_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// spec: DEVICES-001.1
    #[test]
    fn the_name_is_the_last_component_of_the_mount_point() {
        let device = Device::at("/run/media/anna/NO NAME");
        assert_eq!(device.name(), "NO NAME");
        assert_eq!(device.mount_point(), Path::new("/run/media/anna/NO NAME"));
    }

    #[test]
    fn a_mount_point_without_last_component_is_its_own_name() {
        assert_eq!(Device::at("/").name(), "/");
    }

    /// spec: DEVICES-001.3
    #[test]
    fn devices_are_ordered_by_the_bytes_of_the_mount_point() {
        // By components `/a/z` sorts before `/a b`, because `a` < `a b`; by bytes the
        // space (0x20) sorts before the slash (0x2f).
        let mut devices = vec![Device::at("/a/z"), Device::at("/a b")];
        devices.sort();
        assert_eq!(devices, [Device::at("/a b"), Device::at("/a/z")]);
    }
}
