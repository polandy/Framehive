// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What `MemoryFileSystem` simulates beyond the port contract: crashes and faults.

use std::io::{self, Read, Write};
use std::path::Path;

use domain::Timestamp;
use files::testing::{Fault, MemoryFileSystem};
use files::{EntryKind, FileSystem};

const FOLDER: &str = "/destination";

fn file_system() -> MemoryFileSystem {
    let file_system = MemoryFileSystem::new();
    file_system.add_folder(Path::new(FOLDER));
    file_system
}

fn path(name: &str) -> std::path::PathBuf {
    Path::new(FOLDER).join(name)
}

fn read(file_system: &MemoryFileSystem, path: &Path) -> io::Result<Vec<u8>> {
    let mut content = Vec::new();
    file_system.open_read(path)?.read_to_end(&mut content)?;
    Ok(content)
}

#[test]
fn crash_drops_a_file_whose_folder_was_not_synced() {
    let file_system = file_system();
    let mut file = file_system.create_file(&path("photo.raf")).unwrap();
    file.write_all(b"raw").unwrap();
    file.sync().unwrap();

    file_system.crash();

    assert_eq!(file_system.entry(&path("photo.raf")).unwrap(), None);
}

#[test]
fn crash_drops_content_that_was_not_synced() {
    let file_system = file_system();
    let mut file = file_system.create_file(&path("photo.raf")).unwrap();
    file.write_all(b"synced").unwrap();
    file.sync().unwrap();
    file.write_all(b" and lost").unwrap();
    file_system.sync_folder(Path::new(FOLDER)).unwrap();

    file_system.crash();

    assert_eq!(file_system.content(&path("photo.raf")).unwrap(), b"synced");
}

#[test]
fn crash_undoes_a_rename_whose_folder_was_not_synced() {
    let file_system = file_system();
    file_system.add_file(&path("photo.raf.part"), b"raw", Timestamp::UNIX_EPOCH);
    file_system
        .rename_no_replace(&path("photo.raf.part"), &path("photo.raf"))
        .unwrap();

    file_system.crash();

    assert_eq!(file_system.entry(&path("photo.raf")).unwrap(), None);
    assert_eq!(
        file_system.content(&path("photo.raf.part")).unwrap(),
        b"raw"
    );
}

#[test]
fn crash_drops_files_in_a_folder_that_is_not_durable() {
    let file_system = file_system();
    file_system.create_folder(&path("2026")).unwrap();
    let mut file = file_system.create_file(&path("2026/photo.raf")).unwrap();
    file.write_all(b"raw").unwrap();
    file.sync().unwrap();
    file_system.sync_folder(&path("2026")).unwrap();

    file_system.crash();

    assert_eq!(file_system.entry(&path("2026")).unwrap(), None);
    assert_eq!(file_system.paths(), vec![Path::new(FOLDER).to_path_buf()]);
}

#[test]
fn read_fault_fails_after_the_given_bytes() {
    let file_system = file_system();
    file_system.add_file(&path("photo.raf"), b"0123456789", Timestamp::UNIX_EPOCH);
    file_system.inject_fault(&path("photo.raf"), Fault::ReadFails { after_bytes: 4 });

    let mut reader = file_system.open_read(&path("photo.raf")).unwrap();
    let mut start = [0; 4];
    reader.read_exact(&mut start).unwrap();

    assert_eq!(&start, b"0123");
    assert!(reader.read(&mut [0; 4]).is_err());
}

#[test]
fn write_fault_reports_a_full_device() {
    let file_system = file_system();
    file_system.inject_fault(&path("photo.raf"), Fault::WriteFails { after_bytes: 4 });

    let mut file = file_system.create_file(&path("photo.raf")).unwrap();
    let error = file.write_all(b"0123456789").unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::StorageFull);
    assert_eq!(file_system.content(&path("photo.raf")).unwrap(), b"0123");
}

#[test]
fn read_back_fault_returns_different_bytes() {
    let file_system = file_system();
    file_system.add_file(&path("photo.raf"), b"raw", Timestamp::UNIX_EPOCH);
    file_system.inject_fault(&path("photo.raf"), Fault::ReadReturnsDifferentBytes);

    let content = read(&file_system, &path("photo.raf")).unwrap();

    assert_eq!(content.len(), 3);
    assert_ne!(content, b"raw");
    assert_eq!(file_system.content(&path("photo.raf")).unwrap(), b"raw");
}

#[test]
fn other_program_creates_its_file_on_the_first_write() {
    let file_system = file_system();
    let fault = Fault::OtherProgramCreates {
        path: path("photo.raf"),
        content: b"theirs".to_vec(),
    };
    file_system.inject_fault(&path("photo.raf.part"), fault);

    let mut file = file_system.create_file(&path("photo.raf.part")).unwrap();
    assert_eq!(file_system.entry(&path("photo.raf")).unwrap(), None);
    file.write_all(b"ours").unwrap();

    assert_eq!(file_system.content(&path("photo.raf")).unwrap(), b"theirs");
}

#[test]
fn stopped_file_system_fails_every_later_operation() {
    let file_system = file_system();
    file_system.stop_after_operations(1);

    assert_eq!(
        file_system.entry(Path::new(FOLDER)).unwrap(),
        Some(EntryKind::Folder)
    );
    assert!(file_system.entry(Path::new(FOLDER)).is_err());
    assert!(file_system.create_folder(&path("2026")).is_err());
    assert_eq!(file_system.operations(), 1);
}
