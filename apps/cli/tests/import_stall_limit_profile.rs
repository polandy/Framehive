// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `source_stall_limit_seconds` of an import profile (IMPORT-022) against the real
//! binary, with a temporary home folder.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use support::{Folders, stderr, stdout};

/// spec: IMPORT-022.7
#[test]
fn an_invalid_source_stall_limit_stops_the_import() {
    let out_of_range =
        |value: &str| format!("expected a whole number of seconds from 1 to 3600, found {value}");
    let cases = [
        ("3", "0", out_of_range("0")),
        ("3", "3601", out_of_range("3601")),
        ("3", "2.5", out_of_range("2.5")),
        (
            "2",
            "30",
            "source_stall_limit_seconds needs schema_version 3".to_owned(),
        ),
    ];
    for (version, value, message) in cases {
        let folders = Folders::new();
        folders.add_raf();
        let path = folders.write_profile(
            "holiday",
            format!("schema_version: {version}\nsource_stall_limit_seconds: {value}\n").as_bytes(),
        );

        let output = folders.framehive(&[
            "import",
            &folders.source().display().to_string(),
            "--destination",
            &folders.destination().display().to_string(),
            "--profile",
            "holiday",
        ]);

        assert_eq!(
            stderr(&output),
            format!("error: {}:2:29: {message}\n", path.display())
        );
        assert!(stdout(&output).is_empty(), "{}", stdout(&output));
        assert_eq!(output.status.code(), Some(2));
        assert!(!folders.destination().exists());
    }
}
