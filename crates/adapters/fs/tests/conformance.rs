// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: [`StdFileSystem`] passes the conformance suite of the
//! `files::FileSystem` port on the real file system, in a temporary folder (CLAUDE.md,
//! "Adding things").

use adapter_fs::StdFileSystem;
use files::testing::{FileSystemUnderTest, MakeSymbolicLink, TempDir, check_file_system};

#[test]
fn conforms_to_the_file_system_port() {
    let folder = TempDir::new().unwrap();
    let failures = check_file_system(&FileSystemUnderTest {
        file_system: &StdFileSystem::new(),
        root: folder.path(),
        // A crash cannot be simulated on a real file system.
        crash: None,
        make_symbolic_link: SYMBOLIC_LINKS,
    });
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Symbolic links of the platform, for the listing cases of the suite.
#[cfg(unix)]
const SYMBOLIC_LINKS: Option<&MakeSymbolicLink<'static>> =
    Some(&|link, target| std::os::unix::fs::symlink(target, link));

#[cfg(not(unix))]
const SYMBOLIC_LINKS: Option<&MakeSymbolicLink<'static>> = None;
