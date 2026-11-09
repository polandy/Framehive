// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Round trips through exiftool, the version the Nix shell pins (CORE-009, D25).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]
#![allow(
    clippy::expect_used,
    reason = "tests may panic when exiftool is missing"
)]

mod common;

use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Command, Stdio};

use common::{dc, parse_bytes, photoshop, serialize};
use metadata::xmp::XmpPacket;

/// The exiftool of the Nix shell, pinned by `flake.lock`.
const PINNED_VERSION: &str = "13.59";

/// `exiftool` with `arguments`, fed `input`; returns its standard output.
fn exiftool(arguments: &[&str], input: &[u8]) -> Vec<u8> {
    let mut child = Command::new("exiftool")
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("exiftool runs; run the tests inside `nix develop`");
    child.stdin.take().unwrap().write_all(input).unwrap();
    let output = child.wait_with_output().unwrap();
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success() && errors.is_empty(), "{errors}");
    output.stdout
}

/// The XMP tags exiftool reads from `sidecar`, by `group:name`, with structs and
/// lists in exiftool's `-struct` form. Warnings appear as `ExifTool:Warning`.
fn exiftool_tags(sidecar: &[u8]) -> BTreeMap<String, String> {
    let output = exiftool(&["-args", "-G1", "-a", "-struct", "-"], sidecar);
    String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| {
            let (tag, value) = line.trim_start_matches('-').split_once('=').unwrap();
            (tag.to_owned(), value.to_owned())
        })
        .filter(|(tag, _)| {
            let group = tag.split(':').next().unwrap();
            !matches!(group, "System" | "File")
                && !matches!(
                    tag.as_str(),
                    "ExifTool:ExifToolVersion" | "XMP-x:XMPToolkit"
                )
        })
        .collect()
}

/// A new sidecar that exiftool writes with `assignments` such as `-XMP-dc:Creator=…`.
fn written_by_exiftool(assignments: &[&str]) -> Vec<u8> {
    let mut arguments = vec!["-o", "-.xmp"];
    arguments.extend_from_slice(assignments);
    exiftool(&arguments, &[])
}

/// A packet with the import's values, set the way the import sets them.
fn import_values() -> XmpPacket {
    let mut packet = XmpPacket::new();
    packet
        .set_text(&photoshop("TransmissionReference"), "Lisbon 2026")
        .unwrap();
    packet
        .replace_seq(&dc("creator"), &["Ana Example"])
        .unwrap();
    packet
        .set_language(&dc("rights"), "x-default", "© 2026 Ana Example")
        .unwrap();
    packet.add_keywords(&[&keyword("Beach")]).unwrap();
    packet
}

/// The levels of the keyword `Holiday/<leaf>`.
fn keyword(leaf: &str) -> [String; 2] {
    ["Holiday".to_owned(), leaf.to_owned()]
}

fn tags(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|&(tag, value)| (tag.to_owned(), value.to_owned()))
        .collect()
}

#[test]
fn exiftool_is_the_pinned_version() {
    let version = String::from_utf8(exiftool(&["-ver"], &[])).unwrap();
    assert_eq!(version.trim(), PINNED_VERSION);
}

/// spec: CORE-009.22
#[test]
fn exiftool_reads_what_framehive_writes() {
    let written = serialize(&import_values());

    assert_eq!(
        exiftool_tags(written.as_bytes()),
        tags(&[
            ("XMP-dc:Creator", "[Ana Example]"),
            ("XMP-dc:Rights", "© 2026 Ana Example"),
            ("XMP-dc:Subject", "[Beach]"),
            ("XMP-lr:HierarchicalSubject", "[Holiday||Beach]"),
            (
                "XMP-mwg-kw:KeywordInfo",
                "{Hierarchy=[{Children=[{Keyword=Beach}],Keyword=Holiday}]}",
            ),
            ("XMP-photoshop:TransmissionReference", "Lisbon 2026"),
            ("XMP-xmp:MetadataDate", "2026:11:08 14:03:12+01:00"),
        ])
    );
}

/// spec: CORE-009.24
#[test]
fn framehive_reads_what_exiftool_writes() {
    let original = written_by_exiftool(&[
        "-XMP-photoshop:TransmissionReference=Lisbon 2026",
        "-XMP-dc:Creator=Ana Example",
        "-XMP-dc:Rights=© 2026 Ana Example",
        "-XMP-dc:Subject=Beach",
        "-XMP-lr:HierarchicalSubject=Holiday|Beach",
        "-XMP-mwg-kw:KeywordInfo={Hierarchy=[{Keyword=Holiday,Children=[{Keyword=Beach}]}]}",
    ]);

    let mut packet = parse_bytes("exiftool.xmp", &original).unwrap();
    assert_eq!(packet.properties(), import_values().properties());

    packet.add_keywords(&[&keyword("Lisbon")]).unwrap();
    let rewritten = serialize(&packet);

    let mut expected = exiftool_tags(&original);
    expected.extend(tags(&[
        ("XMP-dc:Subject", "[Beach,Lisbon]"),
        (
            "XMP-lr:HierarchicalSubject",
            "[Holiday||Beach,Holiday||Lisbon]",
        ),
        (
            "XMP-mwg-kw:KeywordInfo",
            "{Hierarchy=[{Children=[{Keyword=Beach},{Keyword=Lisbon}],Keyword=Holiday}]}",
        ),
        ("XMP-xmp:MetadataDate", "2026:11:08 14:03:12+01:00"),
    ]));
    assert_eq!(exiftool_tags(rewritten.as_bytes()), expected);
}
