// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! [`Cameras`] that serve [`MemoryFileSystem`]s, for tests.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use domain::Timestamp;

use super::MemoryFileSystem;
use crate::{Camera, CameraError, Cameras, EntryKind, FolderEntry, ReadOnlyFileSystem, ReadSeek};

/// [`Cameras`] that serve [`MemoryFileSystem`]s as cameras, with injectable failures,
/// for tests (ADR 0034 §2).
///
/// A camera's file system is shared with the test, which lays out its storages and
/// files and injects faults through it, for example a camera that stops answering
/// ([`MemoryFileSystem::stop_after_operations`]). An opened camera only reads it.
///
/// ```
/// use std::path::Path;
///
/// use domain::Timestamp;
/// use files::Cameras;
/// use files::testing::{MemoryCameras, MemoryFileSystem};
///
/// let storage = MemoryFileSystem::new();
/// storage.add_file(
///     Path::new("/store_00010001/DCIM/100_FUJI/DSCF0001.RAF"),
///     b"raw",
///     Timestamp::from_unix_seconds(0),
/// );
/// let cameras = MemoryCameras::new().with_camera("Fujifilm X-T5", "usb:002,007", &storage);
///
/// let camera = cameras.open("usb:002,007")?;
/// assert_eq!(camera.file_size(Path::new("/store_00010001/DCIM/100_FUJI/DSCF0001.RAF"))?, 3);
/// assert!(cameras.is_open("usb:002,007"));
/// drop(camera);
/// assert!(!cameras.is_open("usb:002,007"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct MemoryCameras {
    /// The connected cameras by port, so that they list in byte order of their ports.
    cameras: BTreeMap<String, MemoryCamera>,
    list_failure: Option<String>,
    /// How many times each port is open now.
    open: Arc<Mutex<BTreeMap<String, usize>>>,
}

#[derive(Debug, Clone)]
struct MemoryCamera {
    model: String,
    file_system: MemoryFileSystem,
    in_use: Option<String>,
}

impl MemoryCameras {
    /// Creates cameras of which none is connected.
    #[must_use]
    pub fn new() -> Self {
        MemoryCameras::default()
    }

    /// Connects the camera `model` at `port`, whose file system is `file_system`.
    #[must_use]
    pub fn with_camera(mut self, model: &str, port: &str, file_system: &MemoryFileSystem) -> Self {
        self.cameras.insert(
            port.to_owned(),
            MemoryCamera {
                model: model.to_owned(),
                file_system: file_system.clone(),
                in_use: None,
            },
        );
        self
    }

    /// Connects the camera `model` at `port`, which another program holds: it is
    /// listed, and opening it fails with [`CameraError::InUse`] and `reason`.
    #[must_use]
    pub fn with_camera_in_use(mut self, model: &str, port: &str, reason: &str) -> Self {
        self.cameras.insert(
            port.to_owned(),
            MemoryCamera {
                model: model.to_owned(),
                file_system: MemoryFileSystem::new(),
                in_use: Some(reason.to_owned()),
            },
        );
        self
    }

    /// Makes listing the cameras fail with [`CameraError::List`] and `reason`.
    #[must_use]
    pub fn with_list_failure(mut self, reason: &str) -> Self {
        self.list_failure = Some(reason.to_owned());
        self
    }

    /// Whether the camera at `port` is open: opened and not yet released by dropping.
    #[must_use]
    pub fn is_open(&self, port: &str) -> bool {
        lock(&self.open).get(port).is_some_and(|count| *count > 0)
    }
}

impl Cameras for MemoryCameras {
    fn list(&self) -> Result<Vec<Camera>, CameraError> {
        if let Some(reason) = &self.list_failure {
            return Err(CameraError::List {
                reason: reason.clone(),
            });
        }
        Ok(self
            .cameras
            .iter()
            .map(|(port, camera)| Camera {
                model: camera.model.clone(),
                port: port.clone(),
            })
            .collect())
    }

    fn open(&self, port: &str) -> Result<Box<dyn ReadOnlyFileSystem>, CameraError> {
        let camera = self.cameras.get(port).ok_or_else(|| CameraError::Open {
            port: port.to_owned(),
            reason: "no camera is connected at this port".to_owned(),
        })?;
        if let Some(reason) = &camera.in_use {
            return Err(CameraError::InUse {
                port: port.to_owned(),
                reason: reason.clone(),
            });
        }
        *lock(&self.open).entry(port.to_owned()).or_default() += 1;
        Ok(Box::new(OpenedCamera {
            file_system: camera.file_system.clone(),
            port: port.to_owned(),
            open: Arc::clone(&self.open),
        }))
    }
}

/// An opened [`MemoryCamera`]; dropping it releases the camera.
struct OpenedCamera {
    file_system: MemoryFileSystem,
    port: String,
    open: Arc<Mutex<BTreeMap<String, usize>>>,
}

impl Drop for OpenedCamera {
    fn drop(&mut self) {
        if let Some(count) = lock(&self.open).get_mut(&self.port) {
            *count = count.saturating_sub(1);
        }
    }
}

impl ReadOnlyFileSystem for OpenedCamera {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        self.file_system.entry(path)
    }

    fn list_folder(&self, path: &Path) -> io::Result<Vec<FolderEntry>> {
        self.file_system.list_folder(path)
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        self.file_system.file_size(path)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        self.file_system.open_read(path)
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        self.file_system.modification_time(path)
    }
}

/// The open counts; a test that panicked while holding them leaves them usable.
fn lock(open: &Mutex<BTreeMap<String, usize>>) -> MutexGuard<'_, BTreeMap<String, usize>> {
    open.lock().unwrap_or_else(PoisonError::into_inner)
}
