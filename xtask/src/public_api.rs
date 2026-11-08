// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask check-public-api` and `cargo xtask public-api`: the public API of
//! `kernel` and the services as committed text snapshots (architecture §7, ADR 0038).
//!
//! rustdoc writes the API as JSON, and `cargo-public-api` renders that JSON as one
//! sorted line per item. A change to the API must update the snapshot and add a
//! `CHANGELOG.md` entry (`changelog`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::workspace::{self, Package};

pub(crate) mod changelog;

/// The snapshot file in each crate directory.
pub(crate) const SNAPSHOT: &str = "public-api.txt";

/// The layers whose API other crates build on (ADR 0004).
const LAYERS: [&str; 2] = ["kernel", "service"];

/// rustdoc JSON goes to its own target directory: `RUSTC_BOOTSTRAP` is part of Cargo's
/// fingerprint, so sharing `target/` would rebuild the normal build every time.
const TARGET_DIRECTORY: &str = "target/public-api";

/// Compares every snapshot with the current API and checks the changelog rule.
pub(crate) fn check(root: &Path) -> Result<Vec<String>, String> {
    let packages = workspace::load(root)?;
    let mut problems = Vec::new();
    for package in snapshot_crates(&packages) {
        let actual = render(root, &package.name)?;
        let expected = read_snapshot(&package.directory.join(SNAPSHOT))?;
        problems.extend(compare(&package.name, expected.as_deref(), &actual));
    }
    problems.extend(changelog::check(root)?);
    Ok(problems)
}

/// The committed snapshot, or `None` if the crate has none yet.
fn read_snapshot(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

/// Writes the snapshot of every crate; run it after an intended API change.
pub(crate) fn write(root: &Path) -> Result<(), String> {
    let packages = workspace::load(root)?;
    for package in snapshot_crates(&packages) {
        let path = package.directory.join(SNAPSHOT);
        fs::write(&path, render(root, &package.name)?)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

/// The crates of the kernel and service layers, by name.
fn snapshot_crates(packages: &[Package]) -> Vec<&Package> {
    let mut crates: Vec<&Package> = packages
        .iter()
        .filter(|package| {
            package
                .layer
                .as_deref()
                .is_some_and(|layer| LAYERS.contains(&layer))
        })
        .collect();
    crates.sort_by(|left, right| left.name.cmp(&right.name));
    crates
}

/// The public API of one crate, one item per line.
fn render(root: &Path, name: &str) -> Result<String, String> {
    let json = rustdoc_json(root, name)?;
    // Blanket impls such as `impl<T> From<T> for T` hold for every type and only add
    // noise; auto trait impls stay, because losing `Send` breaks callers.
    let output = Command::new("cargo-public-api")
        .arg("--rustdoc-json")
        .arg(&json)
        .args(["--omit", "blanket-impls", "--color", "never"])
        .current_dir(root)
        .output()
        .map_err(|error| missing_tool("cargo-public-api", &error))?;
    if !output.status.success() {
        return Err(format!(
            "cargo-public-api failed for {name}:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| error.to_string())
}

/// Builds the rustdoc JSON of one crate with the pinned stable toolchain.
///
/// The JSON output of rustdoc is unstable; `RUSTC_BOOTSTRAP=1` unlocks it on the stable
/// compiler, so no nightly toolchain is needed (ADR 0038 §2).
fn rustdoc_json(root: &Path, name: &str) -> Result<PathBuf, String> {
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .args(["rustdoc", "--package", name, "--lib", "--locked", "--quiet"])
        .args(["--target-dir", TARGET_DIRECTORY])
        .args(["--", "-Z", "unstable-options", "--output-format", "json"])
        .env("RUSTC_BOOTSTRAP", "1")
        .current_dir(root)
        .output()
        .map_err(|error| missing_tool("cargo", &error))?;
    if !output.status.success() {
        return Err(format!(
            "rustdoc failed for {name}:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(root
        .join(TARGET_DIRECTORY)
        .join("doc")
        .join(format!("{}.json", name.replace('-', "_"))))
}

/// The problem with one crate's snapshot, if any: missing, or the items that differ.
fn compare(name: &str, expected: Option<&str>, actual: &str) -> Option<String> {
    let hint = "if the change is intended, run `cargo xtask public-api` and add a \
                CHANGELOG.md entry";
    let Some(expected) = expected else {
        return Some(format!("{name}: no {SNAPSHOT} yet; {hint}"));
    };
    if expected == actual {
        return None;
    }
    let mut lines = vec![format!(
        "{name}: the public API differs from {SNAPSHOT}; {hint}"
    )];
    lines.extend(only_in(actual, expected).map(|item| format!("  + {item}")));
    lines.extend(only_in(expected, actual).map(|item| format!("  - {item}")));
    Some(lines.join("\n"))
}

/// The lines of `text` that `other` does not have.
fn only_in<'a>(text: &'a str, other: &'a str) -> impl Iterator<Item = &'a str> {
    text.lines()
        .filter(move |line| !other.lines().any(|o| o == *line))
}

fn missing_tool(program: &str, error: &std::io::Error) -> String {
    format!("cannot run {program}: {error}\nhint: run inside `nix develop`")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, layer: Option<&str>) -> Package {
        Package {
            name: name.to_owned(),
            layer: layer.map(str::to_owned),
            port: None,
            ffi: false,
            directory: PathBuf::from(format!("/w/{name}")),
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn snapshots_cover_the_kernel_and_the_services_only() {
        let packages = [
            package("naming", Some("service")),
            package("domain", Some("domain")),
            package("kernel", Some("kernel")),
            package("import", Some("module")),
            package("adapter-sqlite", Some("adapter")),
            package("unlabelled", None),
            package("config", Some("service")),
        ];
        let names: Vec<&str> = snapshot_crates(&packages)
            .iter()
            .map(|package| package.name.as_str())
            .collect();
        assert_eq!(names, ["config", "kernel", "naming"]);
    }

    #[test]
    fn an_unchanged_api_passes() {
        let api = "pub mod kernel\npub struct kernel::ModuleId(_)\n";
        assert_eq!(compare("kernel", Some(api), api), None);
    }

    #[test]
    fn a_missing_snapshot_is_reported() {
        let problem = compare("kernel", None, "pub mod kernel\n").unwrap();
        assert!(
            problem.starts_with("kernel: no public-api.txt yet"),
            "{problem}"
        );
        assert!(problem.contains("cargo xtask public-api"), "{problem}");
    }

    #[test]
    fn a_changed_api_lists_the_added_and_removed_items() {
        let expected = "pub mod kernel\npub fn kernel::old()\n";
        let actual = "pub mod kernel\npub fn kernel::new()\n";
        let problem = compare("kernel", Some(expected), actual).unwrap();
        assert_eq!(
            problem,
            "kernel: the public API differs from public-api.txt; if the change is intended, \
             run `cargo xtask public-api` and add a CHANGELOG.md entry\n  \
             + pub fn kernel::new()\n  - pub fn kernel::old()"
        );
    }
}
