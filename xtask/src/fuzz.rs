// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `cargo xtask fuzz [--seconds <n>]`: coverage-guided fuzzing of our own parsers with
//! cargo-fuzz, inside `nix develop .#fuzz` (ADR 0037).
//!
//! The corpus is not committed (ADR 0017): before each run the seeds are built by the
//! same `metadata::testing` builders the tests use, or copied from the test fixtures,
//! and written into the ignored `fuzz/corpus/<target>/`, next to what earlier runs
//! found.

use std::fs;
use std::path::Path;
use std::process::Command;

use metadata::testing::{
    ByteOrder, DATE_TIME_ORIGINAL, OFFSET_TIME_ORIGINAL, SUB_SEC_TIME_ORIGINAL, Tiff, jpeg, raf,
};

/// The fuzz crate, a workspace of its own (ADR 0037 §2).
pub(crate) const DIRECTORY: &str = "fuzz";

const USAGE: &str = "usage: cargo xtask fuzz [--seconds <n>]";

/// How long each target runs without `--seconds` (ADR 0037 §4).
const DEFAULT_SECONDS: u64 = 600;

/// An input that takes longer is a finding: the parser reads only a few kilobytes.
const INPUT_TIMEOUT_SECONDS: u64 = 10;

/// The XMP sidecars of the `adapter-xmp` tests, the seeds of the `xmp` target.
const XMP_FIXTURES: &str = "crates/adapters/xmp/tests/fixtures";

/// A fuzz target of `fuzz/fuzz_targets/` and the seeds of its corpus.
struct Target {
    name: &'static str,
    seeds: fn(&Path) -> Result<Vec<Seed>, String>,
}

const TARGETS: [Target; 2] = [
    Target {
        name: "capture_time",
        seeds: |_| Ok(capture_time_seeds()),
    },
    Target {
        name: "xmp",
        seeds: xmp_seeds,
    },
];

/// A valid input the fuzzer starts from.
#[derive(Debug)]
struct Seed {
    /// The file name in the corpus; libFuzzer names its own finds by their hash.
    name: String,
    content: Vec<u8>,
}

/// Runs every target for the given time and stops at the first finding.
pub(crate) fn run(root: &Path, arguments: &[String]) -> Result<(), String> {
    let seconds = parse_seconds(arguments)?;
    check_lock(root)?;
    for target in &TARGETS {
        let corpus = root.join(DIRECTORY).join("corpus").join(target.name);
        write_seeds(&corpus, &(target.seeds)(root)?)?;
        run_target(root, target.name, &corpus, seconds)?;
    }
    Ok(())
}

fn parse_seconds(arguments: &[String]) -> Result<u64, String> {
    match arguments {
        [] => Ok(DEFAULT_SECONDS),
        [flag, value] if flag == "--seconds" => match value.parse::<u64>() {
            Ok(seconds) if seconds > 0 => Ok(seconds),
            _ => Err(format!(
                "--seconds needs a whole number above 0, not {value:?}\n{USAGE}"
            )),
        },
        _ => Err(USAGE.to_owned()),
    }
}

/// cargo-fuzz has no `--locked`, so the lock file of the fuzz crate is checked first.
fn check_lock(root: &Path) -> Result<(), String> {
    let manifest = root.join(DIRECTORY).join("Cargo.toml");
    let output = Command::new(cargo())
        .args([
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--manifest-path",
        ])
        .arg(&manifest)
        .output()
        .map_err(|error| format!("cannot run cargo metadata: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "fuzz/Cargo.lock does not match fuzz/Cargo.toml:\n{}",
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// Seeds are rewritten on every run; the corpus files found by earlier runs stay.
fn write_seeds(corpus: &Path, seeds: &[Seed]) -> Result<(), String> {
    fs::create_dir_all(corpus)
        .map_err(|error| format!("cannot create {}: {error}", corpus.display()))?;
    for seed in seeds {
        let path = corpus.join(&seed.name);
        fs::write(&path, &seed.content)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }
    Ok(())
}

/// Runs one target; libFuzzer reports on the terminal and keeps a crashing input in
/// `fuzz/artifacts/<target>/`.
fn run_target(root: &Path, target: &str, corpus: &Path, seconds: u64) -> Result<(), String> {
    println!("fuzz: {target} for {seconds} s");
    let status = Command::new(cargo())
        .args(["fuzz", "run", target])
        .arg(corpus)
        .arg("--")
        .arg(format!("-max_total_time={seconds}"))
        .arg(format!("-timeout={INPUT_TIMEOUT_SECONDS}"))
        .current_dir(root)
        .status()
        .map_err(|error| {
            format!("cannot run cargo fuzz: {error}\nhint: run inside `nix develop .#fuzz`")
        })?;
    if status.success() {
        return Ok(());
    }
    Err(format!(
        "fuzz target {target} failed ({status}); the input is in fuzz/artifacts/{target}/"
    ))
}

/// The cargo that runs xtask, so the toolchain of the shell is used.
fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

/// JPEG, TIFF and RAF, in both byte orders, with and without offset and sub-seconds
/// (ADR 0037 §5).
fn capture_time_seeds() -> Vec<Seed> {
    let mut seeds = Vec::new();
    for (order_name, order) in [
        ("little-endian", ByteOrder::LittleEndian),
        ("big-endian", ByteOrder::BigEndian),
    ] {
        for (detail_name, detailed) in [("plain", false), ("offset-and-sub-seconds", true)] {
            let tiff = tiff(order, detailed);
            let jpeg = jpeg(&tiff);
            let suffix = format!("{order_name}-{detail_name}");
            seeds.push(seed("raf", &suffix, raf(&jpeg)));
            seeds.push(seed("jpeg", &suffix, jpeg));
            seeds.push(seed("tiff", &suffix, tiff));
        }
    }
    seeds
}

/// Every RDF and wrapper form, merges, prefixes, the refused cases (CORE-009) and the
/// sidecars of other programs in subdirectories (CORE-010).
fn xmp_seeds(root: &Path) -> Result<Vec<Seed>, String> {
    let mut seeds = Vec::new();
    add_xmp_seeds(&root.join(XMP_FIXTURES), "", &mut seeds)?;
    seeds.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(seeds)
}

/// Adds every file below `directory`, named by its path with `-` for `/` after `prefix`.
fn add_xmp_seeds(directory: &Path, prefix: &str, seeds: &mut Vec<Seed>) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("cannot list {}: {error}", directory.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("cannot list {}: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            let name = path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            add_xmp_seeds(&path, &format!("{prefix}{name}-"), seeds)?;
            continue;
        }
        let content =
            fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let name = path
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
        seeds.push(seed("xmp", &format!("{prefix}{name}"), content));
    }
    Ok(())
}

fn tiff(order: ByteOrder, detailed: bool) -> Vec<u8> {
    let tiff = Tiff::new(order).with_exif_text(DATE_TIME_ORIGINAL, "2026:10:17 14:05:09");
    if !detailed {
        return tiff.to_bytes();
    }
    tiff.with_exif_text(SUB_SEC_TIME_ORIGINAL, "42")
        .with_exif_text(OFFSET_TIME_ORIGINAL, "+02:00")
        .to_bytes()
}

fn seed(container: &str, suffix: &str, content: Vec<u8>) -> Seed {
    Seed {
        name: format!("seed-{container}-{suffix}"),
        content,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn arguments(text: &[&str]) -> Vec<String> {
        text.iter().map(|&argument| argument.to_owned()).collect()
    }

    #[test]
    fn runs_ten_minutes_without_seconds() {
        assert_eq!(parse_seconds(&[]), Ok(DEFAULT_SECONDS));
    }

    #[test]
    fn takes_the_seconds_given() {
        assert_eq!(parse_seconds(&arguments(&["--seconds", "30"])), Ok(30));
    }

    #[test]
    fn rejects_zero_and_text_as_seconds() {
        assert!(parse_seconds(&arguments(&["--seconds", "0"])).is_err());
        assert!(parse_seconds(&arguments(&["--seconds", "ten"])).is_err());
        assert!(parse_seconds(&arguments(&["--seconds"])).is_err());
        assert!(parse_seconds(&arguments(&["--minutes", "1"])).is_err());
    }

    #[test]
    fn every_capture_time_seed_has_its_own_name() {
        let seeds = capture_time_seeds();
        let names: HashSet<&str> = seeds.iter().map(|seed| seed.name.as_str()).collect();
        assert_eq!(names.len(), 12);
        assert_eq!(names.len(), seeds.len());
    }

    /// A seed the parser rejects would start the fuzzer off the paths that matter.
    #[test]
    fn every_capture_time_seed_has_a_capture_time() {
        for seed in capture_time_seeds() {
            let time = metadata::capture_time_from_bytes(&seed.content);
            assert!(time.is_ok(), "{}: {time:?}", seed.name);
        }
    }

    #[test]
    fn the_xmp_seeds_are_the_fixtures() {
        let seeds = xmp_seeds(&crate::repository_root()).unwrap();
        assert!(seeds.iter().any(|seed| seed.name == "seed-xmp-canonical"));
        assert!(
            seeds
                .iter()
                .any(|seed| seed.name == "seed-xmp-darktable-eos-350d-cosmos.dng")
        );
        assert!(seeds.iter().all(|seed| !seed.content.is_empty()));
    }
}
