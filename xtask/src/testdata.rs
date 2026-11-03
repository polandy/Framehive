// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask testdata <check|fetch>`: the large test files pinned in
//! `testdata.lock` (ADR 0017, ADR 0027).

mod lock;

use std::fs;
use std::path::Path;

const USAGE: &str = "usage: cargo xtask testdata <check|fetch>";

/// Runs the `testdata` subcommand.
pub(crate) fn run(root: &Path, subcommand: Option<&str>) -> Result<(), String> {
    match subcommand {
        Some("check") => check(root),
        _ => Err(USAGE.to_owned()),
    }
}

fn check(root: &Path) -> Result<(), String> {
    let entries = read_lock(root)?;
    println!("testdata.lock: ok, {} entries", entries.len());
    Ok(())
}

fn read_lock(root: &Path) -> Result<Vec<lock::Entry>, String> {
    let path = root.join("testdata.lock");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    lock::parse(&text)
}
