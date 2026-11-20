// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use adapter_fs::StdFileSystem;
use files::testing::{FileSystemUnderTest, TempDir, check_file_system};

use super::{FaultyFileSystem, parse_fault};

/// Without a fault, the decorator conforms to the port as the file system it wraps.
/// spec: IMPORT-004.14
#[test]
fn conforms_to_the_file_system_port_without_a_fault() {
    let folder = TempDir::new().unwrap();
    let failures = check_file_system(&FileSystemUnderTest {
        file_system: &FaultyFileSystem::new(StdFileSystem::new(), None),
        root: folder.path(),
        crash: None,
        make_symbolic_link: Some(&|link, target| std::os::unix::fs::symlink(target, link)),
    });
    assert!(failures.is_empty(), "{failures:#?}");
}

/// A fault without a file name is refused, so a typo never runs without the fault.
/// spec: CLI-025.4
#[test]
fn refuses_a_fault_without_a_file_name() {
    assert_eq!(
        parse_fault("flip-bit-after-sync"),
        Err("FRAMEHIVE_FAULT: `flip-bit-after-sync` needs a file name".to_owned())
    );
}
