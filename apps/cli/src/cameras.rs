// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cameras on the command line (CLI-013, ADR 0034 §5): the `camera:` source of
//! `framehive import`, and `framehive cameras`.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use adapter_gphoto2::Gphoto2Cameras;
use files::{Camera, CameraError, Cameras, ReadOnlyFileSystem};
use import::{ScanError, SourceScan, scan_camera};

use crate::{NOT_STARTED, report_error};

/// What a source that names a camera starts with.
const CAMERA_PREFIX: &str = "camera:";

/// The line when no camera is connected.
const NO_CAMERA: &str = "no camera is connected";

/// Which camera a `camera:` source names (CLI-013, "Naming a camera").
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CameraChoice {
    /// `camera:`: the only connected camera.
    Only,
    /// `camera:<port>`, text with a `:`: the camera at that port, even if it is not
    /// listed.
    Port(String),
    /// `camera:<model>`: the connected camera with that model, ignoring ASCII case.
    Model(String),
}

/// An opened camera: its file system, read only, and its name in the import history.
/// Dropping it releases the camera.
pub(crate) struct OpenedCamera {
    /// The camera's file system, with the root `/` and one folder per storage.
    pub(crate) file_system: Box<dyn ReadOnlyFileSystem>,
    /// `camera:<model>`, as the import log shows the source (CLI-013).
    pub(crate) name: PathBuf,
    /// `the camera <model> at <port>`, or `the camera at <port>` for a camera that is
    /// not listed, as the messages of CLI-013 name it.
    pub(crate) described: String,
}

impl OpenedCamera {
    /// Scans the camera from its root `root` (IMPORT-014).
    ///
    /// # Errors
    ///
    /// Returns the message of the scan error; for a camera without a storage, the
    /// line of CLI-013.15 that names the camera.
    pub(crate) fn scan(&self, root: &Path) -> Result<SourceScan, String> {
        scan_camera(self.file_system.as_ref(), root).map_err(|error| match error {
            ScanError::NoStorage => self.no_storage_message(),
            error => error.to_string(),
        })
    }

    /// The line for a camera that shows no storage, with the remedy (CLI-013.15).
    pub(crate) fn no_storage_message(&self) -> String {
        format!(
            "{} shows no storage; unlock it and choose file transfer (MTP or PTP) on the \
             device, then try again",
            self.described
        )
    }
}

/// The camera that `source` names, or `None` for a folder: a source names a camera
/// when it starts with `camera:`.
pub(crate) fn camera_choice(source: &Path) -> Option<CameraChoice> {
    let rest = source
        .as_os_str()
        .as_encoded_bytes()
        .strip_prefix(CAMERA_PREFIX.as_bytes())?;
    let text = String::from_utf8_lossy(rest).into_owned();
    Some(if text.is_empty() {
        CameraChoice::Only
    } else if text.contains(':') {
        CameraChoice::Port(text)
    } else {
        CameraChoice::Model(text)
    })
}

/// Finds and opens the camera that `choice` names.
///
/// # Errors
///
/// Returns the line of CLI-013 for no camera, several cameras, a camera that cannot be
/// opened, a camera in use, and cameras that cannot be listed.
pub(crate) fn open_camera(
    cameras: &dyn Cameras,
    choice: &CameraChoice,
) -> Result<OpenedCamera, String> {
    let (port, model) = match choice {
        CameraChoice::Only => only_camera(&list(cameras)?)?,
        CameraChoice::Model(model) => camera_with_model(&list(cameras)?, model)?,
        // A port need not be listed, so a failed list does not stop it; the model
        // only names the camera in messages and in the history.
        CameraChoice::Port(port) => (port.clone(), listed_model(cameras, port)),
    };
    match cameras.open(&port) {
        Ok(file_system) => Ok(OpenedCamera {
            file_system,
            name: PathBuf::from(format!(
                "{CAMERA_PREFIX}{}",
                model.as_deref().unwrap_or(&port)
            )),
            described: described_camera(model.as_deref(), &port),
        }),
        Err(CameraError::InUse { .. }) => Err(in_use_message(model.as_deref(), &port)),
        Err(error) => Err(error.to_string()),
    }
}

/// Why a listed camera could not be opened by the guided import (CLI-014).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OpenFailure {
    /// Another program holds the camera: the line of CLI-013 with the remedy.
    InUse(String),
    /// Any other reason, as the driver reports it.
    Other(String),
}

/// Opens the listed camera `model` at `port` for the guided import (CLI-014).
///
/// # Errors
///
/// Returns [`OpenFailure::InUse`] with the line of CLI-013 when another program holds
/// the camera, and [`OpenFailure::Other`] with the reason otherwise.
pub(crate) fn open_listed_camera(
    cameras: &dyn Cameras,
    model: &str,
    port: &str,
) -> Result<OpenedCamera, OpenFailure> {
    match cameras.open(port) {
        Ok(file_system) => Ok(OpenedCamera {
            file_system,
            name: PathBuf::from(format!("{CAMERA_PREFIX}{model}")),
            described: described_camera(Some(model), port),
        }),
        Err(CameraError::InUse { .. }) => {
            Err(OpenFailure::InUse(in_use_message(Some(model), port)))
        }
        Err(CameraError::Open { reason, .. } | CameraError::List { reason }) => {
            Err(OpenFailure::Other(reason))
        }
    }
}

/// The connected cameras.
///
/// # Errors
///
/// Returns the message when they cannot be listed.
fn list(cameras: &dyn Cameras) -> Result<Vec<Camera>, String> {
    cameras.list().map_err(|error| error.to_string())
}

/// The port and model of the only camera in `listed`.
///
/// # Errors
///
/// Returns the line for no camera or several cameras, which names each by port.
fn only_camera(listed: &[Camera]) -> Result<(String, Option<String>), String> {
    match listed {
        [] => Err(NO_CAMERA.to_owned()),
        [camera] => Ok((camera.port.clone(), Some(camera.model.clone()))),
        several => {
            let named = several
                .iter()
                .map(|camera| format!("{CAMERA_PREFIX}{} ({})", camera.port, camera.model))
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!("several cameras are connected; name one: {named}"))
        }
    }
}

/// The port and model of the one camera in `listed` whose model is `model`, ignoring
/// ASCII case.
///
/// # Errors
///
/// Returns the line for no such camera or several.
fn camera_with_model(listed: &[Camera], model: &str) -> Result<(String, Option<String>), String> {
    let matching: Vec<&Camera> = listed
        .iter()
        .filter(|camera| camera.model.eq_ignore_ascii_case(model))
        .collect();
    match matching.as_slice() {
        [] => Err(format!("no camera {model} is connected")),
        [camera] => Ok((camera.port.clone(), Some(camera.model.clone()))),
        several => {
            let ports = several
                .iter()
                .map(|camera| format!("{CAMERA_PREFIX}{}", camera.port))
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!(
                "several cameras {model} are connected; name one: {ports}"
            ))
        }
    }
}

/// The model of the listed camera at `port`, if the cameras can be listed and one is
/// there.
fn listed_model(cameras: &dyn Cameras, port: &str) -> Option<String> {
    cameras
        .list()
        .ok()?
        .into_iter()
        .find(|camera| camera.port == port)
        .map(|camera| camera.model)
}

/// `the camera <model> at <port>`, or `the camera at <port>` without a model.
fn described_camera(model: Option<&str>, port: &str) -> String {
    match model {
        Some(model) => format!("the camera {model} at {port}"),
        None => format!("the camera at {port}"),
    }
}

/// The line for a camera that another program holds, with the remedy (ADR 0034 §6).
fn in_use_message(model: Option<&str>, port: &str) -> String {
    format!(
        "{} is in use by another program; close it \
         (the file manager on Linux, ptpcamerad on macOS) and try again",
        described_camera(model, port)
    )
}

/// Runs `framehive cameras` with libgphoto2 and returns its exit code: 0, or 2 when
/// the cameras cannot be listed.
pub(crate) fn run_cameras() -> ExitCode {
    let result = list_cameras(
        &Gphoto2Cameras::new(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            report_error(&message);
            ExitCode::from(NOT_STARTED)
        }
    }
}

/// Writes one line `<model>  <port>` per connected camera to `output`, or
/// `no camera is connected` to `errors`.
///
/// # Errors
///
/// Returns the message when the cameras cannot be listed.
fn list_cameras(
    cameras: &dyn Cameras,
    output: &mut dyn Write,
    errors: &mut dyn Write,
) -> Result<(), String> {
    let listed = list(cameras)?;
    // Nothing more can be reported when standard output or standard error is gone.
    if listed.is_empty() {
        let _ = writeln!(errors, "{NO_CAMERA}");
    }
    for camera in &listed {
        let _ = writeln!(output, "{}  {}", camera.model, camera.port);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

    use files::testing::{MemoryCameras, MemoryFileSystem};

    use super::*;

    /// The cameras of CLI-013.1.
    fn two_cameras() -> MemoryCameras {
        let storage = MemoryFileSystem::new();
        MemoryCameras::new()
            .with_camera("Fujifilm X-T5", "usb:002,007", &storage)
            .with_camera("Fujifilm X-T2", "usb:001,004", &storage)
    }

    /// `framehive cameras` against `cameras`: its result, standard output and
    /// standard error.
    fn listed(cameras: &MemoryCameras) -> (Result<(), String>, String, String) {
        let (mut output, mut errors) = (Vec::new(), Vec::new());
        let result = list_cameras(cameras, &mut output, &mut errors);
        (
            result,
            String::from_utf8(output).unwrap(),
            String::from_utf8(errors).unwrap(),
        )
    }

    /// spec: CLI-013.1
    #[test]
    fn the_connected_cameras_are_listed_by_port() {
        let (result, output, errors) = listed(&two_cameras());

        assert_eq!(result, Ok(()));
        assert_eq!(
            output,
            "Fujifilm X-T2  usb:001,004\nFujifilm X-T5  usb:002,007\n"
        );
        assert_eq!(errors, "");
    }

    /// spec: CLI-013.2
    #[test]
    fn without_a_camera_the_list_is_empty() {
        let (result, output, errors) = listed(&MemoryCameras::new());

        assert_eq!(result, Ok(()));
        assert_eq!(output, "");
        assert_eq!(errors, "no camera is connected\n");
    }

    #[test]
    fn a_list_that_fails_is_an_error() {
        let (result, output, _) = listed(&MemoryCameras::new().with_list_failure("no USB"));

        assert_eq!(result, Err("cannot list the cameras: no USB".to_owned()));
        assert_eq!(output, "");
    }

    /// spec: CLI-013.13
    #[test]
    fn only_a_source_that_starts_with_camera_names_a_camera() {
        assert_eq!(
            camera_choice(Path::new("camera:")),
            Some(CameraChoice::Only)
        );
        assert_eq!(
            camera_choice(Path::new("camera:usb:001,004")),
            Some(CameraChoice::Port("usb:001,004".to_owned()))
        );
        assert_eq!(
            camera_choice(Path::new("camera:Fujifilm X-T5")),
            Some(CameraChoice::Model("Fujifilm X-T5".to_owned()))
        );
        assert_eq!(camera_choice(Path::new("./camera:x")), None);
        assert_eq!(camera_choice(Path::new("/media/card")), None);
    }

    #[test]
    fn a_port_names_the_camera_by_its_listed_model_or_else_its_port() {
        let storage = MemoryFileSystem::new();
        let cameras = MemoryCameras::new().with_camera("Fujifilm X-T5", "usb:002,007", &storage);
        let listed = open_camera(&cameras, &CameraChoice::Port("usb:002,007".to_owned()));
        assert_eq!(
            listed.map(|camera| camera.name).unwrap(),
            Path::new("camera:Fujifilm X-T5")
        );

        let unlisted = cameras.clone().with_list_failure("no USB");
        let opened = open_camera(&unlisted, &CameraChoice::Port("usb:002,007".to_owned()));
        assert_eq!(
            opened.map(|camera| camera.name).unwrap(),
            Path::new("camera:usb:002,007")
        );
    }
}
