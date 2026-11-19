// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The binary built with the fault-injection point (ADR 0048), run with real adapters.
//! It compiles only with the feature `fault-injection`, which the CI step
//! `fault injection` sets.

#![cfg(feature = "fault-injection")]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use metadata::testing::raf;
use support::{DEFAULT_TARGET, Folders, photo, stderr, summary_line};

/// Without a fault, the import of a binary with the fault-injection point copies as
/// the released binary does.
/// spec: CLI-001.1
#[test]
fn imports_as_usual_without_a_fault() {
    let folders = Folders::new();
    folders.add_raf();

    let output = folders.import(&[]);

    assert_eq!(
        summary_line(&output),
        "1 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out"
    );
    assert_eq!(
        fs::read(folders.destination().join(DEFAULT_TARGET)).unwrap(),
        raf(&photo())
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}
