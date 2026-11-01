// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive` without a subcommand (CLI-002) when it does not run on a terminal.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::process::{Command, Stdio};

use support::{Folders, stderr, stdout};

/// spec: CLI-002.1
#[test]
fn the_guided_import_needs_a_terminal() {
    let folders = Folders::new();
    let state_home = folders.source();
    fs::create_dir_all(state_home.join("framehive")).unwrap();
    fs::write(
        state_home.join("framehive/guided-import.txt"),
        "format_version=7\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_framehive"))
        .env("XDG_STATE_HOME", &state_home)
        .env("HOME", folders.destination())
        .stdin(Stdio::piped())
        .output()
        .unwrap();

    assert_eq!(
        stderr(&output),
        "error: the guided import needs a terminal; use framehive import <source> <destination> in scripts\n"
    );
    assert_eq!(stdout(&output), "");
    assert_eq!(output.status.code(), Some(2));
}
