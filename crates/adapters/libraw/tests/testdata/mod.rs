// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The files of `testdata.lock`, which `cargo xtask testdata fetch` provides.

#![allow(dead_code, reason = "each test binary uses only some of the helpers")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// The `GoPro` HERO11 GPR, a format the pinned LibRaw does not support (ADR 0028).
pub(crate) const GPR: &str = "raw.pixls.us/GoPro/HERO11-Black/GOPR0123.GPR";

/// The test data directory: `$FRAMEHIVE_TESTDATA_DIR`, or `.testdata/` in the
/// repository root, as `cargo xtask testdata fetch` uses it.
fn directory() -> PathBuf {
    env::var_os("FRAMEHIVE_TESTDATA_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.testdata"),
        PathBuf::from,
    )
}

/// The path of `relative` in the test data directory.
///
/// # Panics
///
/// If the file is missing, so that a test never passes without its data
/// (PREVIEWS-001).
pub(crate) fn path(relative: &str) -> PathBuf {
    let path = directory().join(relative);
    assert!(
        path.is_file(),
        "test data {relative} is missing in {}; run `cargo xtask testdata fetch`",
        directory().display()
    );
    path
}

/// A copy of `relative` truncated to half its length, in this test's temporary
/// directory.
pub(crate) fn truncated_copy(relative: &str) -> PathBuf {
    let content = fs::read(path(relative)).unwrap();
    let name = Path::new(relative).file_name().unwrap().to_string_lossy();
    let copy = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("half-{name}"));
    fs::write(&copy, &content[..content.len() / 2]).unwrap();
    copy
}
