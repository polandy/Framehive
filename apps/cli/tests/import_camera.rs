// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import camera:…` against the real binary (CLI-013): a folder served as a
//! camera by libgphoto2's `directory` driver on a `disk:` port, and a folder whose
//! name starts with `camera:`. The child process runs with `TZ=UTC`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use metadata::testing::raf;
use support::{DEFAULT_TARGET, Folders, has_line, photo, stderr, stdout};

/// spec: CLI-013.3
#[test]
fn a_camera_given_by_its_port_is_imported() {
    let folders = Folders::new();
    folders.add("store_00010001/DCIM/100_FUJI/DSCF0001.RAF", &raf(&photo()));
    let source = format!("camera:disk:{}", folders.source().display());
    let destination = folders.destination();

    let output = folders.framehive(&[
        "import",
        &source,
        "--destination",
        destination.to_str().unwrap(),
    ]);

    assert!(
        has_line(
            &output,
            &format!("copied  store_00010001/DCIM/100_FUJI/DSCF0001.RAF -> {DEFAULT_TARGET}")
        ),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(destination.join(DEFAULT_TARGET).is_file());
    assert_eq!(output.status.code(), Some(0));
}

/// spec: CLI-013.13
#[test]
fn a_folder_named_like_a_camera_is_imported_as_a_folder() {
    let folders = Folders::new();
    let card = folders.root.path().join("camera:x");
    fs::create_dir(&card).unwrap();
    fs::write(card.join("DSCF0001.RAF"), raf(&photo())).unwrap();
    let destination = folders.destination();

    let output = folders
        .framehive_command(&[
            "import",
            "./camera:x",
            "--destination",
            destination.to_str().unwrap(),
        ])
        .current_dir(folders.root.path())
        .output()
        .unwrap();

    assert!(
        has_line(
            &output,
            &format!("copied  DSCF0001.RAF -> {DEFAULT_TARGET}")
        ),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(0));
}
