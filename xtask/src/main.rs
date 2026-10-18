// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Repository automation for Framehive: `cargo xtask <command>`.
//!
//! The checks enforce the rules in `CLAUDE.md` so that quality does not depend on
//! human review. See `xtask/README.md`.

#![forbid(unsafe_code)]
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "xtask is a terminal tool, not a library; it reports on stdout and stderr"
)]

mod allowlist;
mod architecture;
mod ci;
mod ci_failures;
mod dependencies;
mod files;
mod json;
mod specs;
mod workspace;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <command>

commands:
  ci                   run every check, exactly as CI does
  check-architecture   layers, unsafe code and crate READMEs (architecture §3)
  check-dependencies   dependencies.yaml allowlist (ADR 0009)
  check-specs          specifications and their tests (ADR 0010)
  check-files          only text files, no binaries (ADR 0017)
  ci-failures [<pr>]   only the failures of a red CI run (needs gh)";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let root = repository_root();
    let result = match arguments.first().map(String::as_str) {
        Some("ci") => ci::run(&root),
        Some("ci-failures") => ci_failures::run(&root, arguments.get(1).map(String::as_str)),
        Some("check-architecture") => check_architecture(&root),
        Some("check-dependencies") => check_dependencies(&root),
        Some("check-specs") => check_specs(&root),
        Some("check-files") => check_files(&root),
        _ => Err(USAGE.to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// xtask lives directly below the repository root.
fn repository_root() -> PathBuf {
    let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_directory
        .parent()
        .unwrap_or(manifest_directory)
        .to_path_buf()
}

fn check_architecture(root: &Path) -> Result<(), String> {
    let packages = workspace::load(root)?;
    finish("architecture", &architecture::check(&packages))
}

fn check_dependencies(root: &Path) -> Result<(), String> {
    let packages = workspace::load(root)?;
    let path = root.join("dependencies.yaml");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let allowlist =
        allowlist::parse(&text).map_err(|error| format!("dependencies.yaml: {error}"))?;
    finish("dependencies", &dependencies::check(&packages, &allowlist))
}

fn check_specs(root: &Path) -> Result<(), String> {
    let report = specs::check_repository(root)?;
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    finish("specs", &report.errors)
}

fn check_files(root: &Path) -> Result<(), String> {
    finish("files", &files::check_repository(root)?)
}

fn finish(check: &str, problems: &[String]) -> Result<(), String> {
    if problems.is_empty() {
        println!("{check}: ok");
        return Ok(());
    }
    for problem in problems {
        eprintln!("error: {problem}");
    }
    Err(format!("{check}: {} problem(s)", problems.len()))
}
