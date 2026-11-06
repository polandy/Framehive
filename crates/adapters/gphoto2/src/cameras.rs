// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! [`Gphoto2Cameras`]: the [`Cameras`] port with libgphoto2 (ADR 0034 §3).

use files::{Camera, CameraError, Cameras, ReadOnlyFileSystem};

use crate::file_system::CameraFileSystem;
use crate::library::{GpError, autodetect};
use crate::session::Session;

/// The ports of libgphoto2's `directory` driver, which serves a folder as a camera.
const DISK_PORT_PREFIX: &str = "disk:";

/// The model of the `directory` driver.
const DIRECTORY_MODEL: &str = "Directory Browse";

/// The cameras and phones connected over PTP or MTP, through libgphoto2.
///
/// It lists only cameras on USB and other device ports. Autodetection also reports a
/// mounted card with a `DCIM` folder as a "Mass Storage Camera" on a `disk:` port; such
/// a card is imported as a folder instead. [`Cameras::open`] still opens a
/// `disk:<folder>` port with the `directory` driver, which serves the folder as a
/// camera, so the whole path to a camera can be tested without hardware.
///
/// ```no_run
/// use adapter_gphoto2::Gphoto2Cameras;
/// use files::Cameras;
///
/// let cameras = Gphoto2Cameras::new();
/// if let Some(camera) = cameras.list()?.first() {
///     let storages = cameras.open(&camera.port)?.list_folder("/".as_ref());
///     assert!(storages.is_ok(), "{} at {}", camera.model, camera.port);
/// }
/// # Ok::<(), files::CameraError>(())
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Gphoto2Cameras;

impl Gphoto2Cameras {
    /// Returns the cameras of libgphoto2.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Cameras for Gphoto2Cameras {
    fn list(&self) -> Result<Vec<Camera>, CameraError> {
        let detected = autodetect().map_err(|error| CameraError::List {
            reason: error.message(),
        })?;
        Ok(cameras_from(detected))
    }

    fn open(&self, port: &str) -> Result<Box<dyn ReadOnlyFileSystem>, CameraError> {
        let model = model_at(port)?;
        let mut session = Session::open(&model, port).map_err(|error| camera_error(port, error))?;
        // Opening succeeds on a missing folder of the `directory` driver; listing the
        // storages shows that the camera answers.
        session
            .list_folders(c"/")
            .map_err(|error| camera_error(port, error))?;
        Ok(Box::new(CameraFileSystem::new(session)))
    }
}

/// The cameras among the autodetected models and ports, sorted by port, each port once.
fn cameras_from(detected: Vec<(String, String)>) -> Vec<Camera> {
    let mut cameras: Vec<Camera> = detected
        .into_iter()
        .filter(|(_, port)| !port.starts_with(DISK_PORT_PREFIX))
        .map(|(model, port)| Camera { model, port })
        .collect();
    cameras.sort_by(|a, b| a.port.cmp(&b.port));
    cameras.dedup_by(|a, b| a.port == b.port);
    cameras
}

/// The model of the camera at `port`. Without a model, libgphoto2 would open the first
/// camera it detects instead.
fn model_at(port: &str) -> Result<String, CameraError> {
    if port.starts_with(DISK_PORT_PREFIX) {
        return Ok(DIRECTORY_MODEL.to_owned());
    }
    let detected = autodetect().map_err(|error| camera_error(port, error))?;
    detected
        .into_iter()
        .find(|(_, detected_port)| detected_port == port)
        .map(|(model, _)| model)
        .ok_or_else(|| CameraError::Open {
            port: port.to_owned(),
            reason: "no camera is connected at this port".to_owned(),
        })
}

/// The error of opening the camera at `port`: busy codes mean another program holds
/// it (ADR 0034 §6).
fn camera_error(port: &str, error: GpError) -> CameraError {
    let port = port.to_owned();
    let reason = error.message();
    if error.is_busy() {
        CameraError::InUse { port, reason }
    } else {
        CameraError::Open { port, reason }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detected(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(model, port)| ((*model).to_owned(), (*port).to_owned()))
            .collect()
    }

    #[test]
    fn cards_on_disk_ports_are_not_listed_as_cameras() {
        let cameras = cameras_from(detected(&[
            ("Mass Storage Camera", "disk:/run/media/card"),
            ("Fujifilm X-T5", "usb:001,005"),
        ]));
        assert_eq!(
            cameras,
            vec![Camera {
                model: "Fujifilm X-T5".into(),
                port: "usb:001,005".into(),
            }]
        );
    }

    #[test]
    fn cameras_are_sorted_by_port_and_listed_once() {
        let cameras = cameras_from(detected(&[
            ("Fujifilm X-T5", "usb:002,003"),
            ("Fujifilm X-T2", "usb:001,005"),
            ("Fujifilm X-T5", "usb:002,003"),
        ]));
        let ports: Vec<&str> = cameras.iter().map(|camera| camera.port.as_str()).collect();
        assert_eq!(ports, ["usb:001,005", "usb:002,003"]);
    }

    #[test]
    fn a_camera_held_by_another_program_is_in_use() {
        for code in [-53, -60, -110] {
            assert!(
                matches!(camera_error("usb:001,005", GpError(code)), CameraError::InUse { port, .. } if port == "usb:001,005"),
                "{code}"
            );
        }
        assert!(matches!(
            camera_error("usb:001,005", GpError(-7)),
            CameraError::Open { .. }
        ));
    }
}
