// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive --help` against the real binary (CLI-023.1).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::process::Command;

/// spec: CLI-023.1
#[test]
fn help_shows_examples_folders_and_exit_codes() {
    let output = Command::new(env!("CARGO_BIN_EXE_framehive"))
        .arg("--help")
        .output()
        .unwrap();

    let help = String::from_utf8_lossy(&output.stdout);
    for line in [
        "Examples:",
        "framehive import /media/card ~/Pictures",
        "Folders:",
        "Configuration:",
        "import history",
        "Exit codes:",
        "  0 ",
        "  1 ",
        "  2 ",
    ] {
        assert!(help.contains(line), "{line} missing in:\n{help}");
    }
    assert_eq!(output.status.code(), Some(0));
}
