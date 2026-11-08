// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask build-release` and `cargo xtask check-reproducible`: release builds of
//! the `framehive` command that are bit-for-bit reproducible (ADR 0007).
//!
//! A release build takes `SOURCE_DATE_EPOCH` from the last commit and remaps the
//! source, Cargo home and target directories to fixed prefixes, so the binary depends
//! on neither the time nor the directory it was built in.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::files;

const PACKAGE: &str = "framehive-cli";
const BINARY: &str = "framehive";

const SOURCE_PREFIX: &str = "/framehive";
const CARGO_HOME_PREFIX: &str = "/cargo";
const TARGET_PREFIX: &str = "/target";

/// Separates the flags in `CARGO_ENCODED_RUSTFLAGS`, so paths may contain spaces.
const FLAG_SEPARATOR: &str = "\u{1f}";

/// Builds the `framehive` command reproducibly into `target/release-build/`.
pub(crate) fn build_release(root: &Path) -> Result<(), String> {
    let binary = build(root, root, &root.join("target/release-build"))?;
    println!("release build: {}", binary.display());
    Ok(())
}

/// Builds the command twice from scratch, the second time from a copy of the sources
/// in another directory, and compares the binaries.
pub(crate) fn check(root: &Path) -> Result<Vec<String>, String> {
    if !cfg!(target_os = "linux") {
        // ADR 0007 requires bit-for-bit equality of the Linux release builds only.
        eprintln!("warning: reproducibility not checked: only Linux release builds are compared");
        return Ok(Vec::new());
    }
    let work = root.join("target/reproducible");
    remove_directory(&work)?;
    let copy = work.join("source");
    copy_sources(root, &copy)?;
    let first = build(root, root, &work.join("first"))?;
    let second = build(root, &copy, &work.join("second"))?;
    let builder_paths = [path_text(root), path_text(&cargo_home())];
    let problems = compare_builds(&read(&first)?, &read(&second)?, &builder_paths);
    // Kept on failure for inspection; otherwise it would only bloat the CI cache of `target`.
    if problems.is_empty() {
        remove_directory(&work)?;
    }
    Ok(problems)
}

/// Runs a release build of `source` into `target` and returns the binary's path.
fn build(root: &Path, source: &Path, target: &Path) -> Result<PathBuf, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(["build", "--release", "--locked", "--package", PACKAGE])
        .arg("--target-dir")
        .arg(target)
        .current_dir(source)
        .env("SOURCE_DATE_EPOCH", commit_time(root)?)
        .env(
            "CARGO_ENCODED_RUSTFLAGS",
            rustflags(source, &cargo_home(), target),
        )
        .env_remove("RUSTFLAGS")
        .output()
        .map_err(|error| format!("cannot run cargo: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "release build of {} failed:\n{}",
            source.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(target.join("release").join(BINARY))
}

/// The remapping flags; rustc applies the last matching one, so the target directory,
/// which may lie inside the source directory, comes last.
fn rustflags(source: &Path, cargo_home: &Path, target: &Path) -> String {
    [
        (source, SOURCE_PREFIX),
        (cargo_home, CARGO_HOME_PREFIX),
        (target, TARGET_PREFIX),
    ]
    .iter()
    .map(|(from, to)| format!("--remap-path-prefix={}={to}", from.display()))
    .collect::<Vec<_>>()
    .join(FLAG_SEPARATOR)
}

/// The commit time of `HEAD` in seconds since the Unix epoch.
fn commit_time(root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%ct"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))?;
    let time = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() || time.is_empty() {
        return Err(format!(
            "cannot read the commit time of HEAD:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(time)
}

/// Cargo's own default: `$CARGO_HOME`, else `$HOME/.cargo`.
fn cargo_home() -> PathBuf {
    std::env::var_os("CARGO_HOME").map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cargo"),
        PathBuf::from,
    )
}

/// Copies every file Git tracks or would track, as `check-files` sees them.
fn copy_sources(root: &Path, destination: &Path) -> Result<(), String> {
    for file in files::repository_files(root)? {
        let from = root.join(&file);
        // Tracked files deleted in the working tree are not part of the build.
        if !from.is_file() {
            continue;
        }
        let to = destination.join(&file);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        fs::copy(&from, &to).map_err(|error| {
            format!(
                "cannot copy {} to {}: {error}",
                from.display(),
                to.display()
            )
        })?;
    }
    Ok(())
}

fn remove_directory(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("cannot remove {}: {error}", path.display()))
        }
        _ => Ok(()),
    }
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

/// Lists why two builds of the same sources are not reproducible.
fn compare_builds(first: &[u8], second: &[u8], builder_paths: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    if let Some(offset) = first_difference(first, second) {
        problems.push(format!(
            "the two release builds differ ({} and {} bytes, first difference at byte {offset})",
            first.len(),
            second.len()
        ));
    }
    for path in builder_paths.iter().filter(|path| !path.is_empty()) {
        if contains(first, path.as_bytes()) {
            problems.push(format!(
                "the release build contains the builder path {path}; remap it with \
                 --remap-path-prefix"
            ));
        }
    }
    problems
}

fn first_difference(first: &[u8], second: &[u8]) -> Option<usize> {
    first
        .iter()
        .zip(second)
        .position(|(a, b)| a != b)
        .or_else(|| (first.len() != second.len()).then(|| first.len().min(second.len())))
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaps_the_target_directory_last() {
        let flags = rustflags(
            Path::new("/home/a/framehive"),
            Path::new("/home/a/.cargo"),
            Path::new("/home/a/framehive/target/x"),
        );
        assert_eq!(
            flags.split(FLAG_SEPARATOR).collect::<Vec<_>>(),
            [
                "--remap-path-prefix=/home/a/framehive=/framehive",
                "--remap-path-prefix=/home/a/.cargo=/cargo",
                "--remap-path-prefix=/home/a/framehive/target/x=/target",
            ]
        );
    }

    #[test]
    fn keeps_spaces_in_paths_inside_one_flag() {
        let flags = rustflags(Path::new("/a b"), Path::new("/c"), Path::new("/d"));
        assert!(
            flags.starts_with("--remap-path-prefix=/a b=/framehive\u{1f}"),
            "{flags}"
        );
    }

    #[test]
    fn equal_builds_without_builder_paths_pass() {
        assert!(
            compare_builds(
                b"ELF /framehive/src",
                b"ELF /framehive/src",
                &["/home/a".to_owned()]
            )
            .is_empty()
        );
    }

    #[test]
    fn reports_the_first_differing_byte() {
        let problems = compare_builds(b"abcd", b"abXd", &[]);
        assert_eq!(
            problems,
            ["the two release builds differ (4 and 4 bytes, first difference at byte 2)"]
        );
    }

    #[test]
    fn a_longer_build_differs_where_the_shorter_ends() {
        assert_eq!(first_difference(b"abc", b"abcd"), Some(3));
    }

    #[test]
    fn reports_a_builder_path_in_equal_builds() {
        // The Nix shell once linked a runpath into the checkout (flake.nix, NIX_NO_SELF_RPATH).
        let binary = b"RUNPATH /home/a/framehive/outputs/out/lib:/nix/store";
        let problems = compare_builds(binary, binary, &["/home/a/framehive".to_owned()]);
        assert_eq!(problems.len(), 1);
        assert!(
            problems[0].contains("builder path /home/a/framehive"),
            "{}",
            problems[0]
        );
    }

    #[test]
    fn ignores_an_empty_builder_path() {
        assert!(compare_builds(b"abc", b"abc", &[String::new()]).is_empty());
    }
}
