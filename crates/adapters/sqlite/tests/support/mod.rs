// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A temporary folder for the tests of this adapter.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_NUMBER: AtomicU32 = AtomicU32::new(0);

/// A fresh folder below the system's temporary folder, removed when dropped.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn new() -> Self {
        let number = NEXT_NUMBER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("framehive-sqlite-test-{}-{number}", process::id()));
        // A folder left behind by an earlier process with the same id is stale.
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path).unwrap();
        TempDir { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
