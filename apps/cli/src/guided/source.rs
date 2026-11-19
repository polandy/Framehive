// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the guided import reads from: a mounted card, or an opened camera (CLI-014).
//! Dropping an opened camera releases it.

use std::io;
use std::path::{Path, PathBuf};

use devices::Device;
use import::{
    DEFAULT_SOURCE_STALL_LIMIT, ImportJob, ImportJobReports, ImportJobSettings, ImportPorts,
    ImportRun, ImportSettings, OwnSource, ScanError, SourceGuard, SourceScan, scan_camera,
    scan_folder,
};

use super::Ports;
use crate::cameras::OpenedCamera;

/// A device the dialog reads: a card at its mount point, or an opened camera from its
/// root `/`. Its scan and plan read it through a stall guard (IMPORT-022).
pub(super) struct Source {
    device: Device,
    root: PathBuf,
    camera: Option<OpenedCamera>,
    guard: SourceGuard,
}

impl Source {
    /// The card `device`, read at `mount_point`.
    ///
    /// # Errors
    ///
    /// Returns an error if the thread that reads the card cannot be started.
    pub(super) fn card(device: &Device, mount_point: &Path, ports: &Ports<'_>) -> io::Result<Self> {
        let guard = ports
            .import
            .guard_source(None, DEFAULT_SOURCE_STALL_LIMIT)?;
        Ok(Source {
            device: device.clone(),
            root: mount_point.to_path_buf(),
            camera: None,
            guard,
        })
    }

    /// The camera `device`, opened as `camera` and read from its root.
    ///
    /// # Errors
    ///
    /// Returns an error if the thread that reads the camera cannot be started.
    pub(super) fn camera(
        device: &Device,
        camera: OpenedCamera,
        ports: &Ports<'_>,
    ) -> io::Result<Self> {
        let own = Box::new(camera.file_system.clone()) as OwnSource;
        let guard = ports
            .import
            .guard_source(Some(own), DEFAULT_SOURCE_STALL_LIMIT)?;
        Ok(Source {
            device: device.clone(),
            root: PathBuf::from("/"),
            camera: Some(camera),
            guard,
        })
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
    pub(super) fn scan(&self) -> Result<SourceScan, ScanError> {
        match &self.camera {
            Some(_) => scan_camera(&self.guard, &self.root),
            None => scan_folder(&self.guard, &self.root),
        }
    }

    /// An import run through `ports` that reads this source; a camera is read through
    /// its own file system and named `camera:<model>` in the history (CLI-013).
    pub(super) fn run<'r>(&'r self, ports: &Ports<'r>) -> ImportRun<'r> {
        let run = ports.import_run();
        match &self.camera {
            Some(camera) => run.with_source(&self.guard).with_source_name(&camera.name),
            None => run.reading_source_through(&self.guard),
        }
    }

    /// An import job through `ports` with `settings` that reads this source as
    /// [`Source::run`] does, and the receiver of its reports (IMPORT-015).
    pub(super) fn import_job(
        &self,
        ports: &ImportPorts,
        settings: &ImportSettings<'_>,
    ) -> (ImportJob, ImportJobReports) {
        let (job, reports) = ImportJob::new(ports.clone(), ImportJobSettings::of(settings));
        let job = match &self.camera {
            Some(camera) => {
                job.with_source(Box::new(camera.file_system.clone()), camera.name.clone())
            }
            None => job,
        };
        (job, reports)
    }
}
