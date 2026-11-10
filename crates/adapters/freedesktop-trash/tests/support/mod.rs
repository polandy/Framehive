// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Helpers shared by the tests of this adapter.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::fs;
use std::path::Path;

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};

/// 2026-11-09 17:05:07 UTC.
pub(crate) const DELETED: i64 = 1_794_243_907;

/// A clock at [`DELETED`], one hour east of UTC.
pub(crate) fn clock() -> ManualClock {
    ManualClock::new(
        Timestamp::from_unix_seconds(DELETED),
        UtcOffset::from_seconds(3_600).unwrap(),
    )
}

/// Writes `path` with `content`, creating its missing parent folders.
pub(crate) fn write_file(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}
