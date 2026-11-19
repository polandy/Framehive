// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive library remove-raws` against the real binary and trash (CLI-018.8,
//! CLI-018.10). On macOS the trash is the one of the user who runs the tests; the
//! in-crate tests with a trash in memory cover it there.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::io::Write;
use std::process::{Output, Stdio};

use support::{Folders, stderr, stdout};

const UNMARKED: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmp:Rating="4"/></rdf:RDF></x:xmpmeta>"#;

fn add_pair_with_sidecars(folders: &Folders, base_name: &str) {
    for extension in ["RAF", "JPG"] {
        let name = format!("{base_name}.{extension}");
        folders.add(&name, b"media");
        folders.add(&format!("{name}.xmp"), UNMARKED.as_bytes());
    }
}

/// Runs `framehive library remove-raws` on the source with `answer` on standard input
/// and the home trash below the root.
fn remove_raws(folders: &Folders, answer: &str) -> Output {
    let folder = folders.source().display().to_string();
    let mut child = folders
        .framehive_command(&["library", "remove-raws", &folder])
        .env("XDG_DATA_HOME", folders.root.path().join("data"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(answer.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// spec: CLI-018.8
#[cfg(not(target_os = "macos"))]
#[test]
fn a_file_that_cannot_be_moved_is_reported() {
    let folders = Folders::new();
    // The info file of the trash adds 10 characters, so the name of `A` is too long for it.
    let long_name = "A".repeat(245);
    add_pair_with_sidecars(&folders, &long_name);
    add_pair_with_sidecars(&folders, "B");

    let output = remove_raws(&folders, "yes\n");

    let folder = folders.source().display().to_string();
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stderr(&output).starts_with(&format!(
            "error: cannot move {folder}/{long_name}.RAF to the trash: "
        )),
        "{}",
        stderr(&output)
    );
    assert_eq!(stderr(&output).lines().count(), 1);
    for name in [format!("{long_name}.RAF"), format!("{long_name}.RAF.xmp")] {
        assert!(folders.source().join(&name).exists(), "{name}");
    }
    let mut trashed: Vec<String> = fs::read_dir(folders.root.path().join("data/Trash/files"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    trashed.sort();
    assert_eq!(trashed, ["B.RAF", "B.RAF.xmp"]);
    assert!(stdout(&output).ends_with("Moved to the trash: 1 asset, 2 files, 0.0 MB freed\n"));
}

/// spec: CLI-018.10
#[test]
fn an_unreadable_sidecar_keeps_its_asset_out() {
    let folders = Folders::new();
    add_pair_with_sidecars(&folders, "A");
    fs::write(folders.source().join("A.JPG.xmp"), b"not XMP").unwrap();
    add_pair_with_sidecars(&folders, "B");

    let output = remove_raws(&folders, "n\n");

    let folder = folders.source().display().to_string();
    assert!(
        stderr(&output).starts_with(&format!(
            "warning: the XMP sidecar {folder}/A.JPG.xmp cannot be read: "
        )),
        "{}",
        stderr(&output)
    );
    assert!(stderr(&output).ends_with("; its asset is left out\n"));
    assert!(stdout(&output).contains("  RAW will be removed (1): B\n"));
}
