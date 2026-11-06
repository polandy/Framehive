// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The test doubles pass the conformance suites of their ports.

use std::path::Path;

use files::testing::{
    CamerasUnderTest, FileSystemUnderTest, FnvHasher, FreeSpaceUnderTest, HasherUnderTest,
    MemoryCameras, MemoryFileSystem, MemoryFreeSpace, TempDir, check_cameras, check_file_system,
    check_free_space, check_hasher,
};
use files::{CameraError, Cameras, FreeSpace};

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
fn memory_cameras_conform() {
    let storage = MemoryFileSystem::new();
    let cameras = MemoryCameras::new()
        .with_camera("Fujifilm X-T2", "usb:001,004", &storage)
        .with_camera_in_use("Fujifilm X-T5", "usb:002,007", "the device is busy");
    let add_folder = |path: &Path| {
        storage.add_folder(path);
        Ok(())
    };
    let add_file = |path: &Path, content: &[u8], modified| {
        storage.add_file(path, content, modified);
        Ok(())
    };

    let failures = check_cameras(&CamerasUnderTest {
        cameras: &cameras,
        port: "usb:001,004",
        missing_port: "usb:009,009",
        add_folder: &add_folder,
        add_file: &add_file,
    });

    assert_eq!(failures, Vec::<String>::new());
    assert!(!cameras.is_open("usb:001,004"));
}

#[test]
fn memory_cameras_list_by_port_and_tell_a_camera_in_use() {
    let cameras = MemoryCameras::new()
        .with_camera_in_use("Fujifilm X-T5", "usb:002,007", "the device is busy")
        .with_camera("Fujifilm X-T2", "usb:001,004", &MemoryFileSystem::new());

    let ports: Vec<String> = cameras
        .list()
        .unwrap()
        .into_iter()
        .map(|c| c.port)
        .collect();
    let in_use = cameras.open("usb:002,007").err();

    assert_eq!(ports, ["usb:001,004", "usb:002,007"]);
    assert_eq!(
        in_use,
        Some(CameraError::InUse {
            port: "usb:002,007".to_owned(),
            reason: "the device is busy".to_owned(),
        })
    );
    assert!(!cameras.is_open("usb:002,007"));
}

#[test]
fn memory_cameras_fail_to_list_when_told() {
    let cameras = MemoryCameras::new().with_list_failure("no USB access");

    assert_eq!(
        cameras.list().unwrap_err().to_string(),
        "cannot list the cameras: no USB access"
    );
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
