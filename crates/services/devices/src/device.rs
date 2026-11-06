// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A device as it is offered for import.

use std::cmp::Ordering;
use std::fmt;
use std::path::{Path, PathBuf};

/// A device as it is offered for import (glossary): a memory card mounted as a file
/// system, identified by its mount point, or a camera connected over PTP or MTP,
/// identified by its model and port.
///
/// Devices are ordered with all cards before all cameras; cards by the bytes of
/// their mount points (DEVICES-001.3), which is not the component order of [`Path`],
/// and cameras by the bytes of their ports, then by model (DEVICES-001.16).
///
/// ```
/// use devices::Device;
///
/// let card = Device::at("/run/media/anna/EOS_DIGITAL");
/// assert_eq!(card.name(), "EOS_DIGITAL");
/// assert_eq!(card.to_string(), "EOS_DIGITAL (/run/media/anna/EOS_DIGITAL)");
///
/// let camera = Device::camera("Fujifilm X-T5", "usb:002,007");
/// assert_eq!(camera.to_string(), "Fujifilm X-T5 (camera at usb:002,007)");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Device {
    name: String,
    kind: DeviceKind,
}

/// What identifies a [`Device`] and how it is reached.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    /// A memory card mounted as a file system.
    Card {
        /// Where the card is mounted; it identifies the card while it is mounted.
        mount_point: PathBuf,
    },
    /// A camera or phone connected over PTP or MTP.
    Camera {
        /// Where the camera is connected, such as `usb:002,007`. A camera that is
        /// plugged in again gets a new port.
        port: String,
    },
}

impl Device {
    /// The card mounted at `mount_point`; its name is the last component of the
    /// mount point, or the whole mount point if it has no last component, such as `/`.
    #[must_use]
    pub fn at(mount_point: impl Into<PathBuf>) -> Self {
        let mount_point = mount_point.into();
        let name = mount_point
            .file_name()
            .unwrap_or(mount_point.as_os_str())
            .to_string_lossy()
            .into_owned();
        Device {
            name,
            kind: DeviceKind::Card { mount_point },
        }
    }

    /// The camera of `model` connected at `port`; its name is the model.
    #[must_use]
    pub fn camera(model: impl Into<String>, port: impl Into<String>) -> Self {
        Device {
            name: model.into(),
            kind: DeviceKind::Camera { port: port.into() },
        }
    }

    /// The name shown to the user, such as `EOS_DIGITAL` or `Fujifilm X-T5`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the device is a card or a camera, and what identifies it.
    #[must_use]
    pub fn kind(&self) -> &DeviceKind {
        &self.kind
    }

    /// Where the card is mounted; `None` for a camera.
    #[must_use]
    pub fn mount_point(&self) -> Option<&Path> {
        match &self.kind {
            DeviceKind::Card { mount_point } => Some(mount_point),
            DeviceKind::Camera { .. } => None,
        }
    }

    /// Where the camera is connected; `None` for a card.
    #[must_use]
    pub fn port(&self) -> Option<&str> {
        match &self.kind {
            DeviceKind::Card { .. } => None,
            DeviceKind::Camera { port } => Some(port),
        }
    }

    /// Whether the device is a camera.
    #[must_use]
    pub fn is_camera(&self) -> bool {
        matches!(self.kind, DeviceKind::Camera { .. })
    }
}

impl Ord for Device {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.kind, &other.kind) {
            (DeviceKind::Card { mount_point: left }, DeviceKind::Card { mount_point: right }) => {
                left.as_os_str()
                    .as_encoded_bytes()
                    .cmp(right.as_os_str().as_encoded_bytes())
            }
            (DeviceKind::Card { .. }, DeviceKind::Camera { .. }) => Ordering::Less,
            (DeviceKind::Camera { .. }, DeviceKind::Card { .. }) => Ordering::Greater,
            (DeviceKind::Camera { port: left }, DeviceKind::Camera { port: right }) => left
                .as_bytes()
                .cmp(right.as_bytes())
                .then_with(|| self.name.as_bytes().cmp(other.name.as_bytes())),
        }
    }
}

impl PartialOrd for Device {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Device {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            DeviceKind::Card { mount_point } => {
                write!(formatter, "{} ({})", self.name, mount_point.display())
            }
            DeviceKind::Camera { port } => {
                write!(formatter, "{} (camera at {port})", self.name)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// spec: DEVICES-001.1
    #[test]
    fn the_name_is_the_last_component_of_the_mount_point() {
        let device = Device::at("/run/media/anna/NO NAME");
        assert_eq!(device.name(), "NO NAME");
        assert_eq!(
            device.mount_point(),
            Some(Path::new("/run/media/anna/NO NAME"))
        );
        assert_eq!(device.port(), None);
        assert!(!device.is_camera());
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

    /// spec: DEVICES-001.15
    #[test]
    fn a_camera_is_named_by_its_model_and_identified_by_its_port() {
        let camera = Device::camera("Fujifilm X-T5", "usb:002,007");
        assert_eq!(camera.name(), "Fujifilm X-T5");
        assert_eq!(camera.port(), Some("usb:002,007"));
        assert_eq!(camera.mount_point(), None);
        assert!(camera.is_camera());
        assert_eq!(
            camera.kind(),
            &DeviceKind::Camera {
                port: "usb:002,007".to_owned()
            }
        );
        assert_eq!(camera.to_string(), "Fujifilm X-T5 (camera at usb:002,007)");
    }

    /// spec: DEVICES-001.16
    #[test]
    fn cards_sort_before_cameras_and_cameras_by_port_then_model() {
        let mut devices = vec![
            Device::camera("B", "usb:001,009"),
            Device::camera("Z", "usb:001,005"),
            Device::at("/z"),
            Device::camera("A", "usb:001,009"),
            Device::at("/a"),
        ];
        devices.sort();
        assert_eq!(
            devices,
            [
                Device::at("/a"),
                Device::at("/z"),
                Device::camera("Z", "usb:001,005"),
                Device::camera("A", "usb:001,009"),
                Device::camera("B", "usb:001,009"),
            ]
        );
    }
}
