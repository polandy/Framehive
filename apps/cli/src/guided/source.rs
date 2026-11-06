// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the guided import reads from: a mounted card, or an opened camera (CLI-014).
//! Dropping an opened camera releases it.

use std::path::{Path, PathBuf};

use devices::Device;
use import::{ImportRun, ScanError, SourceScan, scan_camera, scan_folder};

use super::Ports;
use crate::cameras::OpenedCamera;

/// A device the dialog reads: a card at its mount point, or an opened camera from its
/// root `/`.
pub(super) struct Source {
    device: Device,
    root: PathBuf,
    camera: Option<OpenedCamera>,
}

impl Source {
    /// The card `device`, read at `mount_point`.
    pub(super) fn card(device: &Device, mount_point: &Path) -> Self {
        Source {
            device: device.clone(),
            root: mount_point.to_path_buf(),
            camera: None,
        }
    }

    /// The camera `device`, opened as `camera` and read from its root.
    pub(super) fn camera(device: &Device, camera: OpenedCamera) -> Self {
        Source {
            device: device.clone(),
            root: PathBuf::from("/"),
            camera: Some(camera),
        }
    }

    /// The device this source reads.
    pub(super) fn device(&self) -> &Device {
        &self.device
    }

    /// The folder the import reads from: the mount point, or `/` of a camera.
    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    /// The opened camera, for a camera.
    pub(super) fn opened_camera(&self) -> Option<&OpenedCamera> {
        self.camera.as_ref()
    }

    /// Whether this source is a camera.
    pub(super) fn is_camera(&self) -> bool {
        self.camera.is_some()
    }

    /// How the format readiness names the source (CLI-007, CLI-014).
    pub(super) fn noun(&self) -> &'static str {
        if self.is_camera() { "camera" } else { "card" }
    }

    /// Scans the files to import (IMPORT-001, IMPORT-014).
    ///
    /// # Errors
    ///
    /// Returns the scan error; [`ScanError::NoStorage`] for a camera without a storage.
    pub(super) fn scan(&self, ports: &Ports<'_>) -> Result<SourceScan, ScanError> {
        match &self.camera {
            Some(camera) => scan_camera(camera.file_system.as_ref(), &self.root),
            None => scan_folder(ports.file_system, &self.root),
        }
    }

    /// An import run through `ports` that reads this source; a camera is read through
    /// its own file system and named `camera:<model>` in the history (CLI-013).
    pub(super) fn run<'r>(&'r self, ports: &Ports<'r>) -> ImportRun<'r> {
        let run = ports.import_run();
        match &self.camera {
            Some(camera) => run
                .with_source(camera.file_system.as_ref())
                .with_source_name(&camera.name),
            None => run,
        }
    }
}
