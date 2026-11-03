// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! These tests download from `file://` URLs to files they create, never from the
//! network (ADR 0006). They need `curl`, `b3sum` and GNU `tar` from the Nix shell.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::super::lock::Entry;
use super::{Outcome, STAMP, blake3, fetch};

/// A fresh directory with a `source/` to download from and a `cache/` to fill.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "framehive-xtask-testdata-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("source")).expect("create the source directory");
        Self { root }
    }

    fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    fn source(&self, name: &str) -> PathBuf {
        self.root.join("source").join(name)
    }

    /// Writes a source file and returns the entry that pins it at `path`.
    fn pinned_file(&self, name: &str, contents: &str, path: &str) -> Entry {
        let source = self.source(name);
        fs::write(&source, contents).expect("write the source file");
        entry_for(&source, path)
    }

    /// Packs `files` into a `.tar.zst` source and returns the entry that pins it.
    fn pinned_archive(&self, name: &str, files: &[(&str, &str)], path: &str) -> Entry {
        let contents = self.root.join(format!("contents-{name}"));
        fs::create_dir_all(&contents).expect("create the archive contents");
        for (file, text) in files {
            fs::write(contents.join(file), text).expect("write an archive file");
        }
        let archive = self.source(name);
        let status = Command::new("tar")
            .args(["--zstd", "--create", "--file"])
            .arg(&archive)
            .arg("--directory")
            .arg(&contents)
            .arg(".")
            .status()
            .expect("run tar");
        assert!(status.success(), "tar failed");
        entry_for(&archive, path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn entry_for(source: &Path, path: &str) -> Entry {
    Entry {
        blake3: blake3(source).expect("hash the source"),
        size_bytes: fs::metadata(source).expect("stat the source").len(),
        path: path.to_owned(),
        url: format!("file://{}", source.display()),
    }
}

#[test]
fn fetches_a_file_once() {
    let scratch = Scratch::new("file-once");
    let entry = scratch.pinned_file("a.dng", "raw data", "dng/a.dng");

    assert_eq!(fetch(&scratch.cache(), &entry), Ok(Outcome::Fetched));
    assert_eq!(fetch(&scratch.cache(), &entry), Ok(Outcome::UpToDate));

    let target = scratch.cache().join("dng/a.dng");
    assert_eq!(fs::read_to_string(target).expect("read"), "raw data");
    assert!(!scratch.cache().join("dng/a.dng.part").exists());
}

#[test]
fn replaces_a_changed_cached_file() {
    let scratch = Scratch::new("file-changed");
    let entry = scratch.pinned_file("a.dng", "raw data", "a.dng");
    fs::create_dir_all(scratch.cache()).expect("create the cache");
    // Same size, different content: only the hash can tell.
    fs::write(scratch.cache().join("a.dng"), "raw dat!").expect("corrupt");

    assert_eq!(fetch(&scratch.cache(), &entry), Ok(Outcome::Fetched));
    let target = scratch.cache().join("a.dng");
    assert_eq!(fs::read_to_string(target).expect("read"), "raw data");
}

#[test]
fn rejects_a_wrong_hash_and_keeps_nothing() {
    let scratch = Scratch::new("file-hash");
    let mut entry = scratch.pinned_file("a.dng", "raw data", "a.dng");
    let actual = entry.blake3.clone();
    entry.blake3 = "0".repeat(64);

    let error = fetch(&scratch.cache(), &entry).expect_err("wrong hash");
    assert!(error.contains(&format!("blake3 {actual}")), "{error}");
    assert!(error.contains(&format!("pins {}", entry.blake3)), "{error}");
    assert!(!scratch.cache().join("a.dng").exists());
    assert!(!scratch.cache().join("a.dng.part").exists());
}

#[test]
fn rejects_a_wrong_size() {
    let scratch = Scratch::new("file-size");
    let mut entry = scratch.pinned_file("a.dng", "raw data", "a.dng");
    entry.size_bytes += 1;

    let error = fetch(&scratch.cache(), &entry).expect_err("wrong size");
    assert!(
        error.contains("size 8 bytes, but testdata.lock pins 9 bytes"),
        "{error}"
    );
    assert!(!scratch.cache().join("a.dng.part").exists());
}

#[test]
fn reports_a_failed_download() {
    let scratch = Scratch::new("file-missing");
    let mut entry = scratch.pinned_file("a.dng", "raw data", "a.dng");
    entry.url = format!("file://{}", scratch.source("missing.dng").display());

    let error = fetch(&scratch.cache(), &entry).expect_err("missing source");
    assert!(error.starts_with("curl failed"), "{error}");
}

#[test]
fn unpacks_an_archive_once() {
    let scratch = Scratch::new("archive-once");
    let files = [("a.raf", "first"), ("clean.yaml", "steps: []")];
    let entry = scratch.pinned_archive("fujifilm-v1.tar.zst", &files, "fujifilm/");

    assert_eq!(fetch(&scratch.cache(), &entry), Ok(Outcome::Fetched));
    assert_eq!(fetch(&scratch.cache(), &entry), Ok(Outcome::UpToDate));

    let directory = scratch.cache().join("fujifilm");
    assert_eq!(
        fs::read_to_string(directory.join("a.raf")).expect("read"),
        "first"
    );
    let stamp = fs::read_to_string(directory.join(STAMP)).expect("read the stamp");
    assert_eq!(stamp.trim(), entry.blake3);
    assert!(!scratch.cache().join("fujifilm.tar.zst.part").exists());
}

#[test]
fn a_new_archive_version_leaves_no_stale_file() {
    let scratch = Scratch::new("archive-version");
    let old = scratch.pinned_archive("v1.tar.zst", &[("old.raf", "1")], "fujifilm/");
    let new = scratch.pinned_archive("v2.tar.zst", &[("new.raf", "2")], "fujifilm/");

    assert_eq!(fetch(&scratch.cache(), &old), Ok(Outcome::Fetched));
    assert_eq!(fetch(&scratch.cache(), &new), Ok(Outcome::Fetched));

    let directory = scratch.cache().join("fujifilm");
    assert!(directory.join("new.raf").exists());
    assert!(!directory.join("old.raf").exists());
}

#[test]
fn a_broken_archive_leaves_no_directory() {
    let scratch = Scratch::new("archive-broken");
    // A verified download that is not a .tar.zst.
    let entry = scratch.pinned_file("broken.tar.zst", "not an archive", "fujifilm/");

    let error = fetch(&scratch.cache(), &entry).expect_err("broken archive");
    assert!(error.starts_with("tar failed"), "{error}");
    assert!(!scratch.cache().join("fujifilm").exists());
}
