// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The parts of the harness that the dialog and the import job share (IMPORT-015).

use std::mem;
use std::sync::{Mutex, MutexGuard, PoisonError};

use files::{Camera, CameraError, Cameras, ReadOnlyFileSystem};

use super::support::Harness;

/// A value of the world behind a lock, used like a `Cell` or a `RefCell`.
pub(super) struct Shared<T>(Mutex<T>);

impl<T> Shared<T> {
    pub(super) fn new(value: T) -> Self {
        Shared(Mutex::new(value))
    }

    /// The value, locked; a test that panicked while holding it poisons nothing else.
    pub(super) fn borrow(&self) -> MutexGuard<'_, T> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn borrow_mut(&self) -> MutexGuard<'_, T> {
        self.borrow()
    }

    pub(super) fn set(&self, value: T) {
        *self.borrow() = value;
    }

    pub(super) fn replace(&self, value: T) -> T {
        mem::replace(&mut *self.borrow(), value)
    }
}

impl<T: Copy> Shared<T> {
    pub(super) fn get(&self) -> T {
        *self.borrow()
    }
}

impl<T: Default> Shared<T> {
    pub(super) fn take(&self) -> T {
        self.replace(T::default())
    }
}

/// The cameras of the harness as they are now.
pub(super) struct HarnessCameras<'h> {
    pub(super) harness: &'h Harness,
}

impl Cameras for HarnessCameras<'_> {
    fn list(&self) -> Result<Vec<Camera>, CameraError> {
        self.harness.cameras.borrow().list()
    }

    fn open(&self, port: &str) -> Result<Box<dyn ReadOnlyFileSystem + Send>, CameraError> {
        self.harness.cameras.borrow().open(port)
    }
}
