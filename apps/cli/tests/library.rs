// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive library keep-raw` and `framehive library remove-raws` (CLI-018) against
//! the real binary, on a folder and a trash in a temporary folder.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::io::{ErrorKind, Write};
use std::process::{Output, Stdio};

use support::{Folders, lines, stderr, stdout};

const MARKED: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:framehive="https://polandy.github.io/framehive/ns/1.0/" framehive:KeepRaw="True"/></rdf:RDF></x:xmpmeta>"#;
const REJECTED: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmp:Rating="-1"/></rdf:RDF></x:xmpmeta>"#;
const UNMARKED: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmp:Rating="4"/></rdf:RDF></x:xmpmeta>"#;

/// The size of `A.RAF`: with its small sidecar it rounds to 3.0 MB.
const RAW_BYTES: usize = 2_950_000;

/// The folder of CLI-018.5: `A` to be removed, `B` marked, `C` JPEG only, `D` RAW only
/// and `E` rejected.
fn summary_folder(folders: &Folders) {
    folders.add("A.RAF", &vec![0; RAW_BYTES]);
    folders.add("A.JPG", b"jpeg");
    folders.add("A.RAF.xmp", UNMARKED.as_bytes());
    folders.add("B.RAF", b"raw");
    folders.add("B.JPG", b"jpeg");
    folders.add("B.xmp", MARKED.as_bytes());
    folders.add("C.JPG", b"jpeg");
    folders.add("D.RAF", b"raw");
    folders.add("E.RAF", b"raw");
    folders.add("E.JPG", b"jpeg");
    folders.add("E.xmp", REJECTED.as_bytes());
}

/// Runs `framehive library <arguments>` with `input` on standard input and the home
/// trash below the root.
fn library(folders: &Folders, arguments: &[&str], input: &[u8]) -> Output {
    let mut command = folders.framehive_command(&[&["library"], arguments].concat());
    let mut child = command
        .env("XDG_DATA_HOME", folders.root.path().join("data"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // A run that asks nothing may exit before it reads, which closes the pipe.
    if let Err(error) = child.stdin.take().unwrap().write_all(input) {
        assert_eq!(error.kind(), ErrorKind::BrokenPipe, "{error}");
    }
    child.wait_with_output().unwrap()
}

fn source_file(folders: &Folders, name: &str) -> String {
    folders.source().join(name).display().to_string()
}

fn trashed(folders: &Folders) -> Vec<String> {
    let trash = folders.root.path().join("data/Trash/files");
    let Ok(entries) = fs::read_dir(trash) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn sidecar(folders: &Folders, name: &str) -> String {
    fs::read_to_string(folders.source().join(name)).unwrap()
}

fn in_source(folders: &Folders, names: &[&str]) -> bool {
    names
        .iter()
        .all(|name| folders.source().join(name).exists())
}

fn last_line(output: &Output) -> String {
    lines(output).last().cloned().unwrap_or_default()
}

fn asset_line(folders: &Folders, state: &str, base_name: &str) -> String {
    let asset = folders.source().join(base_name);
    format!("Keep RAW {state}: {}\n", asset.display())
}

/// spec: CLI-018.1
#[test]
fn keep_raw_marks_every_member() {
    let folders = Folders::new();
    folders.add("A.RAF", b"raw");
    folders.add("A.JPG", b"jpeg");

    let output = library(
        &folders,
        &["keep-raw", &source_file(&folders, "A.JPG")],
        b"",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), asset_line(&folders, "on", "A"));
    assert!(sidecar(&folders, "A.RAF.xmp").contains("KeepRaw"));
    assert!(sidecar(&folders, "A.JPG.xmp").contains("KeepRaw"));
}

/// spec: CLI-018.2
#[test]
fn keep_raw_off_unmarks() {
    let folders = Folders::new();
    folders.add("A.RAF", b"raw");
    folders.add("A.JPG", b"jpeg");
    let raw = source_file(&folders, "A.RAF");
    library(&folders, &["keep-raw", &raw], b"");

    let output = library(&folders, &["keep-raw", &raw, "--off"], b"");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), asset_line(&folders, "off", "A"));
    assert!(!sidecar(&folders, "A.RAF.xmp").contains("KeepRaw"));
    assert!(!sidecar(&folders, "A.JPG.xmp").contains("KeepRaw"));
}

/// spec: CLI-018.3
#[test]
fn a_file_outside_the_library_changes_nothing() {
    let folders = Folders::new();
    folders.add("A.RAF", b"raw");
    folders.add("A.JPG", b"jpeg");
    folders.add("A.xmp", MARKED.as_bytes());
    folders.add("notes.txt", b"notes");
    let notes = source_file(&folders, "notes.txt");

    let output = library(
        &folders,
        &["keep-raw", &source_file(&folders, "A.RAF"), &notes, "--off"],
        b"",
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr(&output),
        format!("error: {notes} is not a photo or video of the library\n")
    );
    assert_eq!(sidecar(&folders, "A.xmp"), MARKED);
}

/// spec: CLI-018.5
#[test]
fn the_dry_run_prints_the_summary_and_the_files() {
    let folders = Folders::new();
    summary_folder(&folders);

    let output = library(
        &folders,
        &[
            "remove-raws",
            &folders.source().display().to_string(),
            "--dry-run",
        ],
        b"",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        lines(&output),
        [
            "Remove RAWs: 1 asset, 2 files, 3.0 MB freed".to_owned(),
            "  RAW will be removed (1): A".to_owned(),
            "  RAW stays (Keep RAW) (1): B".to_owned(),
            "  Skipped: 1 rejected, 1 JPEG only, 1 RAW only".to_owned(),
            format!("    {}", source_file(&folders, "A.RAF")),
            format!("    {}", source_file(&folders, "A.RAF.xmp")),
        ]
    );
    assert!(trashed(&folders).is_empty());
}

/// spec: CLI-018.6
// On macOS the run would use the Trash of the user who runs the tests; the in-crate
// test with a trash in memory covers it there.
#[cfg(not(target_os = "macos"))]
#[test]
fn confirming_moves_the_raws_to_the_trash() {
    let folders = Folders::new();
    summary_folder(&folders);

    let output = library(
        &folders,
        &["remove-raws", &folders.source().display().to_string()],
        b"y\n",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(trashed(&folders), ["A.RAF", "A.RAF.xmp"]);
    assert!(in_source(
        &folders,
        &[
            "A.JPG", "B.RAF", "B.JPG", "B.xmp", "C.JPG", "D.RAF", "E.RAF", "E.JPG", "E.xmp"
        ]
    ));
    // Piped input echoes no newline, so the last line starts with the question.
    assert_eq!(
        last_line(&output),
        "Move 2 files to the trash? [y/N] Moved to the trash: 1 asset, 2 files, 3.0 MB freed"
    );
}

/// spec: CLI-018.7
#[test]
fn any_other_answer_moves_nothing() {
    let folders = Folders::new();
    summary_folder(&folders);
    let folder = folders.source().display().to_string();

    let output = library(&folders, &["remove-raws", &folder], b"n\n");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(last_line(&output).ends_with("Nothing moved."));
    assert!(trashed(&folders).is_empty());
    assert!(in_source(&folders, &["A.RAF", "A.RAF.xmp"]));
}

/// spec: CLI-018.11
#[test]
fn no_answer_is_an_error() {
    let folders = Folders::new();
    summary_folder(&folders);
    let folder = folders.source().display().to_string();

    let output = library(&folders, &["remove-raws", &folder], b"");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr(&output),
        format!(
            "error: no answer to the question; nothing moved\n\
             hint: framehive library remove-raws --yes {folder}\n"
        )
    );
    assert!(!stdout(&output).contains("Nothing moved."));
    assert!(trashed(&folders).is_empty());
    assert!(in_source(&folders, &["A.RAF", "A.RAF.xmp"]));
}

/// spec: CLI-018.12
// On macOS the run would use the Trash of the user who runs the tests; the in-crate
// test with a trash in memory covers it there.
#[cfg(not(target_os = "macos"))]
#[test]
fn yes_moves_without_asking() {
    let folders = Folders::new();
    summary_folder(&folders);

    let output = library(
        &folders,
        &[
            "remove-raws",
            &folders.source().display().to_string(),
            "--yes",
        ],
        b"",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(trashed(&folders), ["A.RAF", "A.RAF.xmp"]);
    assert!(!stdout(&output).contains("[y/N]"));
    assert_eq!(
        last_line(&output),
        "Moved to the trash: 1 asset, 2 files, 3.0 MB freed"
    );
}

/// spec: CLI-018.9
#[test]
fn nothing_to_remove_asks_nothing() {
    let folders = Folders::new();
    folders.add("C.JPG", b"jpeg");

    let output = library(
        &folders,
        &["remove-raws", &folders.source().display().to_string()],
        b"y\n",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        lines(&output),
        [
            "Remove RAWs: 0 assets, 0 files, 0.0 MB freed",
            "  RAW will be removed (0): -",
            "  RAW stays (Keep RAW) (0): -",
            "  Skipped: 0 rejected, 1 JPEG only, 0 RAW only",
            "Nothing to remove.",
        ]
    );
}

#[test]
fn a_missing_folder_does_not_start() {
    let folders = Folders::new();
    let missing = folders.root.path().join("missing").display().to_string();

    let output = library(&folders, &["remove-raws", &missing], b"");

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).starts_with(&format!("error: cannot read {missing}: ")));
}

/// Asset `A` with `sidecar` as its `A.JPG.xmp`, and the plain asset `B`.
fn folder_with_a_bad_sidecar(folders: &Folders, sidecar: &[u8]) {
    folders.add("A.RAF", b"raw");
    folders.add("A.JPG", b"jpeg");
    folders.add("A.JPG.xmp", sidecar);
    folders.add("B.RAF", b"raw");
    folders.add("B.JPG", b"jpeg");
}

fn remove_raws_answering_no(folders: &Folders) -> Output {
    let folder = folders.source().display().to_string();
    library(folders, &["remove-raws", &folder], b"n\n")
}

/// spec: CORE-009.25
#[test]
fn a_deeply_nested_sidecar_keeps_its_asset_out() {
    let folders = Folders::new();
    let depth = 100_000;
    folder_with_a_bad_sidecar(
        &folders,
        format!("{}{}", "<a>".repeat(depth), "</a>".repeat(depth)).as_bytes(),
    );

    let output = remove_raws_answering_no(&folders);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("nested too deeply"),
        "{}",
        stderr(&output)
    );
    assert!(
        lines(&output).contains(&"  RAW will be removed (1): B".to_owned()),
        "{}",
        stdout(&output)
    );
}

/// spec: CORE-009.26
#[test]
fn a_sidecar_larger_than_16_mib_keeps_its_asset_out() {
    let folders = Folders::new();
    let mut sidecar = UNMARKED.as_bytes().to_vec();
    sidecar.resize(16 * 1024 * 1024 + 1, b' ');
    folder_with_a_bad_sidecar(&folders, &sidecar);

    let output = remove_raws_answering_no(&folders);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("larger than 16 MiB"),
        "{}",
        stderr(&output)
    );
    assert!(
        lines(&output).contains(&"  RAW will be removed (1): B".to_owned()),
        "{}",
        stdout(&output)
    );
}
