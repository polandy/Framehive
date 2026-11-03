// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Downloads, verifies and unpacks the entries of `testdata.lock` into the cache
//! (ADR 0027 §3, §4).
//!
//! It calls `curl`, `b3sum` and GNU `tar` from the Nix shell instead of linking Rust
//! crates for HTTPS, BLAKE3 and Zstandard (ADR 0027 §1).

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::lock::Entry;

/// Written into an unpacked archive directory; holds the hash of its archive.
const STAMP: &str = ".framehive-testdata-blake3";

/// What fetching one entry did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Fetched,
    UpToDate,
}

/// Brings one entry of the lock into the cache.
pub(crate) fn fetch(cache: &Path, entry: &Entry) -> Result<Outcome, String> {
    if entry.is_archive() {
        fetch_archive(cache, entry)
    } else {
        fetch_file(cache, entry)
    }
}

fn fetch_file(cache: &Path, entry: &Entry) -> Result<Outcome, String> {
    let target = cache.join(&entry.path);
    if file_matches(&target, entry)? {
        return Ok(Outcome::UpToDate);
    }
    let download = with_suffix(&target, ".part");
    download_verified(entry, &download)?;
    fs::rename(&download, &target)
        .map_err(|error| format!("cannot move {}: {error}", download.display()))?;
    Ok(Outcome::Fetched)
}

fn fetch_archive(cache: &Path, entry: &Entry) -> Result<Outcome, String> {
    let directory = cache.join(entry.path.trim_end_matches('/'));
    let stamp = directory.join(STAMP);
    if fs::read_to_string(&stamp).is_ok_and(|hash| hash.trim() == entry.blake3) {
        return Ok(Outcome::UpToDate);
    }
    let download = with_suffix(&directory, ".tar.zst.part");
    download_verified(entry, &download)?;
    let unpacked = unpack(&download, &directory);
    remove_file_if_present(&download)?;
    if let Err(error) = unpacked {
        // A half-unpacked directory must not look like an older, complete version.
        remove_directory_if_present(&directory)?;
        return Err(error);
    }
    fs::write(&stamp, format!("{}\n", entry.blake3))
        .map_err(|error| format!("cannot write {}: {error}", stamp.display()))?;
    Ok(Outcome::Fetched)
}

fn file_matches(path: &Path, entry: &Entry) -> Result<bool, String> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.len() == entry.size_bytes => Ok(blake3(path)? == entry.blake3),
        Ok(_) => Ok(false),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

/// Downloads to `part` and keeps the file only if its size and hash match.
fn download_verified(entry: &Entry, part: &Path) -> Result<(), String> {
    let parent = part.parent().unwrap_or(part);
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    let verified = download(&entry.url, part).and_then(|()| verify(entry, part));
    if verified.is_err() {
        remove_file_if_present(part)?;
    }
    verified
}

fn download(url: &str, target: &Path) -> Result<(), String> {
    // The lock allows only https; file is accepted for the tests (ADR 0027).
    let mut command = Command::new("curl");
    command
        .args(["--fail", "--silent", "--show-error", "--location"])
        .args(["--proto", "=https,file", "--output"])
        .arg(target)
        .arg(url);
    run(command, "curl").map(drop)
}

fn verify(entry: &Entry, path: &Path) -> Result<(), String> {
    let size_bytes = fs::metadata(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?
        .len();
    if size_bytes != entry.size_bytes {
        return Err(format!(
            "{}: size {size_bytes} bytes, but testdata.lock pins {} bytes",
            entry.url, entry.size_bytes
        ));
    }
    let hash = blake3(path)?;
    if hash != entry.blake3 {
        return Err(format!(
            "{}: blake3 {hash}, but testdata.lock pins {}",
            entry.url, entry.blake3
        ));
    }
    Ok(())
}

fn blake3(path: &Path) -> Result<String, String> {
    let mut command = Command::new("b3sum");
    command.arg("--no-names").arg(path);
    let output = run(command, "b3sum")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Replaces `directory` with the contents of the archive.
fn unpack(archive: &Path, directory: &Path) -> Result<(), String> {
    remove_directory_if_present(directory)?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let mut command = Command::new("tar");
    command
        .args(["--zstd", "--extract", "--file"])
        .arg(archive)
        .arg("--directory")
        .arg(directory);
    run(command, "tar").map(drop)
}

fn run(mut command: Command, program: &str) -> Result<Output, String> {
    let output = command.output().map_err(|error| match error.kind() {
        ErrorKind::NotFound => format!("{program} not found; run inside `nix develop`"),
        _ => format!("cannot run {program}: {error}"),
    })?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "{program} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn remove_file_if_present(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != ErrorKind::NotFound => {
            Err(format!("cannot remove {}: {error}", path.display()))
        }
        _ => Ok(()),
    }
}

fn remove_directory_if_present(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Err(error) if error.kind() != ErrorKind::NotFound => {
            Err(format!("cannot remove {}: {error}", path.display()))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests;
