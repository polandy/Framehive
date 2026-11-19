// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive library keep-raw` against the real binary when a sidecar refuses the
//! marker (CLI-018.4).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use support::{Folders, stderr, stdout};

const MARKED_ARRAY: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:framehive="https://polandy.github.io/framehive/ns/1.0/"><framehive:KeepRaw><rdf:Bag><rdf:li>True</rdf:li></rdf:Bag></framehive:KeepRaw></rdf:Description></rdf:RDF></x:xmpmeta>"#;

/// spec: CLI-018.4
#[test]
fn a_refused_marker_stops_only_its_asset() {
    let folders = Folders::new();
    folders.add("A.RAF", b"raw");
    folders.add("A.RAF.xmp", MARKED_ARRAY.as_bytes());
    let raw_of_b = folders.add("B.RAF", b"raw");
    let raw_of_a = folders.source().join("A.RAF");

    let output = folders.framehive(&[
        "library",
        "keep-raw",
        &raw_of_a.display().to_string(),
        &raw_of_b.display().to_string(),
    ]);

    let folder = folders.source().display().to_string();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).starts_with(&format!(
            "error: cannot set Keep RAW of {folder}/A: the XMP sidecar {folder}/A.RAF.xmp cannot be changed: "
        )),
        "{}",
        stderr(&output)
    );
    assert_eq!(stdout(&output), format!("Keep RAW on: {folder}/B\n"));
    assert!(folders.source().join("B.RAF.xmp").is_file());
}
