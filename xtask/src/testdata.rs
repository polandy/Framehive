// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask testdata <check|fetch>`: the large test files pinned in
//! `testdata.lock` (ADR 0017, ADR 0027).

mod fetch;
mod lock;

use std::fs;
use std::path::{Path, PathBuf};

const USAGE: &str = "usage: cargo xtask testdata <check|fetch>";

/// Runs the `testdata` subcommand.
pub(crate) fn run(root: &Path, subcommand: Option<&str>) -> Result<(), String> {
    match subcommand {
        Some("check") => check(root),
        Some("fetch") => fetch_all(root),
        _ => Err(USAGE.to_owned()),
    }
}

fn check(root: &Path) -> Result<(), String> {
    let entries = read_lock(root)?;
    println!("testdata.lock: ok, {} entries", entries.len());
    Ok(())
}

/// Fetches every entry and stops at the first failure; there are no retries
/// (ADR 0027 §4).
fn fetch_all(root: &Path) -> Result<(), String> {
    let entries = read_lock(root)?;
    let cache = cache_directory(root);
    for entry in &entries {
        let outcome = fetch::fetch(&cache, entry)?;
        let state = match outcome {
            fetch::Outcome::Fetched => "fetched",
            fetch::Outcome::UpToDate => "up to date",
        };
        println!("{state}: {}", entry.path);
    }
    println!(
        "test data: {} entries in {}",
        entries.len(),
        cache.display()
    );
    Ok(())
}

/// `FRAMEHIVE_TESTDATA_DIR`, or `.testdata/` in the repository root (ADR 0017 §3).
fn cache_directory(root: &Path) -> PathBuf {
    std::env::var_os("FRAMEHIVE_TESTDATA_DIR")
        .filter(|directory| !directory.is_empty())
        .map_or_else(|| root.join(".testdata"), PathBuf::from)
}

fn read_lock(root: &Path) -> Result<Vec<lock::Entry>, String> {
    let path = root.join("testdata.lock");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    lock::parse(&text)
}
