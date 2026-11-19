// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive library remove-raws` against the real binary when the folder changes while
//! the question is open (CLI-018.14). On macOS the run would use the Trash of the user
//! who runs the tests; the in-crate test with a trash in memory covers it there.

#![cfg(not(target_os = "macos"))]
#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::io::{Read, Write};
use std::process::{Child, ChildStdout, Stdio};

use support::{Folders, stderr};

const UNMARKED: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmp:Rating="4"/></rdf:RDF></x:xmpmeta>"#;

/// Starts `framehive library remove-raws <folder>` with the home trash below the root.
fn start_remove_raws(folders: &Folders, folder: &str) -> Child {
    folders
        .framehive_command(&["library", "remove-raws", folder])
        .env("XDG_DATA_HOME", folders.root.path().join("data"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

/// Reads standard output until the question; the command flushes it before it reads.
fn wait_for_question(stdout: &mut ChildStdout) {
    let mut shown = Vec::new();
    let mut byte = [0];
    while !shown.ends_with(b"[y/N] ") {
        stdout.read_exact(&mut byte).unwrap();
        shown.push(byte[0]);
    }
}

fn trashed(folders: &Folders) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folders.root.path().join("data/Trash/files"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// spec: CLI-018.14
#[test]
fn an_asset_that_changed_since_the_summary_keeps_its_raw() {
    let folders = Folders::new();
    for base_name in ["A", "B"] {
        for name in ["RAF", "JPG"].map(|extension| format!("{base_name}.{extension}")) {
            folders.add(&name, b"media");
            folders.add(&format!("{name}.xmp"), UNMARKED.as_bytes());
        }
    }
    let folder = folders.source().display().to_string();
    let mut child = start_remove_raws(&folders, &folder);
    let mut stdout = child.stdout.take().unwrap();
    wait_for_question(&mut stdout);

    fs::remove_file(folders.source().join("A.JPG")).unwrap();
    child.stdin.take().unwrap().write_all(b"y\n").unwrap();
    let mut rest = String::new();
    stdout.read_to_string(&mut rest).unwrap();
    let output = child.wait_with_output().unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        format!(
            "warning: {folder}/A changed since the summary: its rendered file is gone; \
             its RAW stays\n"
        )
    );
    for name in ["A.RAF", "A.RAF.xmp"] {
        assert!(folders.source().join(name).exists(), "{name}");
    }
    assert_eq!(trashed(&folders), ["B.RAF", "B.RAF.xmp"]);
    assert!(rest.ends_with("Moved to the trash: 1 asset, 2 files, 0.0 MB freed\n"));
}
