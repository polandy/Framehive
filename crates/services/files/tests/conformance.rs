// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The test doubles pass the conformance suites of their ports.

use std::path::Path;

use files::FreeSpace;
use files::testing::{
    FileSystemUnderTest, FnvHasher, FreeSpaceUnderTest, HasherUnderTest, MemoryFileSystem,
    MemoryFreeSpace, TempDir, check_file_system, check_free_space, check_hasher,
};

#[test]
fn memory_file_system_conforms() {
    let file_system = MemoryFileSystem::new();
    let root = Path::new("/conformance");
    file_system.add_folder(root);
    let crash = || file_system.crash();
    let make_symbolic_link = |link: &Path, _target: &Path| {
        file_system.add_symbolic_link(link);
        Ok(())
    };

    let failures = check_file_system(&FileSystemUnderTest {
        file_system: &file_system,
        root,
        crash: Some(&crash),
        make_symbolic_link: Some(&make_symbolic_link),
    });

    assert_eq!(failures, Vec::<String>::new());
}

#[test]
fn fnv_hasher_conforms() {
    // Reference values of 64-bit FNV-1a from its authors' test suite.
    let known: [(&[u8], &str); 3] = [
        (b"", "fnv:cbf29ce484222325"),
        (b"a", "fnv:af63dc4c8601ec8c"),
        (b"foobar", "fnv:85944171f73967e8"),
    ];

    let failures = check_hasher(&HasherUnderTest {
        hasher: &FnvHasher,
        algorithm: FnvHasher::ALGORITHM,
        known_checksums: &known,
    });

    assert_eq!(failures, Vec::<String>::new());
}

#[test]
fn memory_free_space_conforms() {
    let free_space = MemoryFreeSpace::new().with_file_system("/", 1_000);

    let failures = check_free_space(&FreeSpaceUnderTest {
        free_space: &free_space,
        folder: Path::new("/conformance"),
    });

    assert_eq!(failures, Vec::<String>::new());
}

#[test]
fn memory_free_space_answers_by_the_nearest_mount() {
    let free_space = MemoryFreeSpace::new()
        .with_file_system("/", 1_000)
        .with_file_system("/media/disk", 500)
        .with_unreadable("/media/share", "the file system does not report it");

    let root = free_space.free_space(Path::new("/home")).unwrap();
    let disk = free_space.free_space(Path::new("/media/disk/a/b")).unwrap();
    let share = free_space.free_space(Path::new("/media/share/library"));

    assert_eq!((root.available_bytes, disk.available_bytes), (1_000, 500));
    assert_ne!(root.file_system, disk.file_system);
    assert_eq!(
        share.unwrap_err().to_string(),
        "the file system does not report it"
    );
    assert!(MemoryFreeSpace::new().free_space(Path::new("/")).is_err());
}

#[test]
fn temp_dir_is_empty_and_removed_with_its_content() {
    let folder = TempDir::new().unwrap();
    let path = folder.path().to_path_buf();
    assert_eq!(std::fs::read_dir(&path).unwrap().count(), 0);
    std::fs::create_dir(path.join("2026")).unwrap();
    std::fs::write(path.join("2026/DSCF0001.RAF"), b"raw").unwrap();

    drop(folder);

    assert!(!path.exists());
}

#[test]
fn temp_dirs_get_distinct_paths() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    assert_ne!(first.path(), second.path());
}
