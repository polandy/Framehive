// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `Cameras` port: cameras and phones connected over PTP or MTP (ADR 0034 §2).

use crate::ReadOnlyFileSystem;

/// A camera or phone connected over PTP or MTP, as [`Cameras::list`] returns it
/// (glossary: Camera).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Camera {
    /// The model name the camera reports, for example `Fujifilm X-T5`.
    pub model: String,
    /// Where the camera is connected, for example `usb:001,005`.
    pub port: String,
}

/// Why the cameras could not be listed or a camera could not be opened.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CameraError {
    /// Another program holds the camera, such as the file manager's gvfs on Linux or
    /// `ptpcamerad` on macOS. Framehive never stops it (ADR 0034 §6).
    #[error("the camera at {port} is in use by another program: {reason}")]
    InUse {
        /// The port of the camera.
        port: String,
        /// What the driver reports.
        reason: String,
    },
    /// The camera cannot be opened for any other reason, for example because no
    /// camera is connected at the port.
    #[error("cannot open the camera at {port}: {reason}")]
    Open {
        /// The port that was opened.
        port: String,
        /// What the driver reports.
        reason: String,
    },
    /// The connected cameras cannot be listed.
    #[error("cannot list the cameras: {reason}")]
    List {
        /// What the driver reports.
        reason: String,
    },
}

/// Port for the cameras and phones connected over PTP or MTP (ADR 0034 §2).
///
/// `adapter-gphoto2` implements it with libgphoto2; tests use
/// [`testing::MemoryCameras`](crate::testing::MemoryCameras). Every implementation
/// must pass [`testing::check_cameras`](crate::testing::check_cameras).
///
/// An opened camera is a [`ReadOnlyFileSystem`] with the root `/` and one folder per
/// storage, named as the camera reports it (for example `store_00010001`). Dropping
/// it releases the camera.
pub trait Cameras {
    /// Returns the connected cameras, sorted by port in byte order.
    ///
    /// # Errors
    ///
    /// Returns [`CameraError::List`] if the cameras cannot be listed.
    fn list(&self) -> Result<Vec<Camera>, CameraError>;

    /// Opens the camera at `port`, even if [`Cameras::list`] does not list it.
    ///
    /// # Errors
    ///
    /// Returns [`CameraError::InUse`] if another program holds the camera, and
    /// [`CameraError::Open`] if it cannot be opened for another reason.
    fn open(&self, port: &str) -> Result<Box<dyn ReadOnlyFileSystem + Send>, CameraError>;
}
