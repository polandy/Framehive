// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the freedesktop.org trash writes beside the moved file, and what it never
//! replaces.

use std::fs;

use adapter_freedesktop_trash::FreedesktopTrash;
use files::Trash;
use files::testing::TempDir;

mod support;

use support::{clock, write_file};

#[test]
fn the_info_file_names_the_original_path_and_the_local_deletion_time() {
    let folder = TempDir::new().unwrap();
    let photo = folder.path().join("DSCF0001.RAF");
    fs::write(&photo, b"raw").unwrap();
    let trash = FreedesktopTrash::new(folder.path().join("Trash"), clock());

    trash.move_to_trash(&photo).unwrap();

    let info = fs::read_to_string(folder.path().join("Trash/info/DSCF0001.RAF.trashinfo")).unwrap();
    let expected = format!(
        "[Trash Info]\nPath={}\nDeletionDate=2026-11-09T18:05:07\n",
        photo.display()
    );
    assert_eq!(info, expected);
}

#[test]
fn a_second_file_of_the_same_name_gets_its_own_info_file() {
    let folder = TempDir::new().unwrap();
    write_file(&folder.path().join("a/DSCF0001.RAF"), b"a").unwrap();
    write_file(&folder.path().join("b/DSCF0001.RAF"), b"b").unwrap();
    let trash = FreedesktopTrash::new(folder.path().join("Trash"), clock());

    trash
        .move_to_trash(&folder.path().join("a/DSCF0001.RAF"))
        .unwrap();
    let second = trash
        .move_to_trash(&folder.path().join("b/DSCF0001.RAF"))
        .unwrap();

    assert_eq!(second, folder.path().join("Trash/files/DSCF0001.2.RAF"));
    let info =
        fs::read_to_string(folder.path().join("Trash/info/DSCF0001.2.RAF.trashinfo")).unwrap();
    assert!(info.contains("/b/DSCF0001.RAF\n"), "{info}");
}

#[test]
fn a_file_left_in_the_trash_without_an_info_file_is_not_replaced() {
    let folder = TempDir::new().unwrap();
    write_file(&folder.path().join("Trash/files/DSCF0001.RAF"), b"orphan").unwrap();
    let photo = folder.path().join("DSCF0001.RAF");
    fs::write(&photo, b"raw").unwrap();
    let trash = FreedesktopTrash::new(folder.path().join("Trash"), clock());

    let in_trash = trash.move_to_trash(&photo).unwrap();

    assert_eq!(in_trash, folder.path().join("Trash/files/DSCF0001.2.RAF"));
    assert_eq!(
        fs::read(folder.path().join("Trash/files/DSCF0001.RAF")).unwrap(),
        b"orphan"
    );
    assert!(
        !folder
            .path()
            .join("Trash/info/DSCF0001.RAF.trashinfo")
            .exists()
    );
}

#[test]
fn a_symbolic_link_is_refused_and_stays() {
    let folder = TempDir::new().unwrap();
    let link = folder.path().join("link.RAF");
    std::os::unix::fs::symlink("DSCF0001.RAF", &link).unwrap();
    let trash = FreedesktopTrash::new(folder.path().join("Trash"), clock());

    let error = trash.move_to_trash(&link).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(fs::symlink_metadata(&link).is_ok());
}
