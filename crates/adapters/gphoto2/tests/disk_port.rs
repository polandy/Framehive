// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the `Cameras` conformance suite leaves to the adapter: opening a missing
//! folder on a `disk:` port, and reading a file in more than one chunk (ADR 0034 §3).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod support;

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use adapter_gphoto2::Gphoto2Cameras;
use domain::Timestamp;
use files::{CameraError, Cameras};

/// Larger than two chunks of the reader, and not a multiple of one.
const LARGE_FILE_BYTES: usize = 2 * 1024 * 1024 + 4_321;

fn large_content() -> Vec<u8> {
    (0..LARGE_FILE_BYTES)
        .map(|index| u8::try_from(index % 251).unwrap())
        .collect()
}

#[test]
fn a_missing_folder_on_a_disk_port_does_not_open() {
    let camera = support::FolderCamera::new();
    let port = format!("{}/missing", camera.port());
    match Gphoto2Cameras::new().open(&port) {
        Err(CameraError::Open { port: failed, .. }) => assert_eq!(failed, port),
        Err(error) => panic!("expected CameraError::Open, got {error:?}"),
        Ok(_) => panic!("opened {port}"),
    }
}

#[test]
fn a_file_larger_than_a_chunk_reads_whole_and_from_any_offset() {
    let camera = support::FolderCamera::new();
    let content = large_content();
    camera.add_folder(Path::new("/store_00010001")).unwrap();
    let path = Path::new("/store_00010001/MOV_0001.MP4");
    camera
        .add_file(path, &content, Timestamp::from_unix_seconds(0))
        .unwrap();
    let opened = Gphoto2Cameras::new().open(&camera.port()).unwrap();
    let mut reader = opened.open_read(path).unwrap();
    drop(opened);

    let mut whole = Vec::new();
    reader.read_to_end(&mut whole).unwrap();
    assert!(whole == content, "the file read differs from its content");

    let across_chunks = 1024 * 1024 - 3;
    reader.seek(SeekFrom::Start(across_chunks)).unwrap();
    let mut bytes = [0; 8];
    reader.read_exact(&mut bytes).unwrap();
    let start = usize::try_from(across_chunks).unwrap();
    assert_eq!(bytes, content[start..start + 8]);
}

#[test]
fn a_file_that_ends_before_its_reported_size_is_an_error_not_a_short_file() {
    let camera = support::FolderCamera::new();
    camera.add_folder(Path::new("/store_00010001")).unwrap();
    let path = Path::new("/store_00010001/IMG_0001.JPG");
    camera
        .add_file(path, &[7; 4_096], Timestamp::from_unix_seconds(0))
        .unwrap();
    let opened = Gphoto2Cameras::new().open(&camera.port()).unwrap();
    let mut reader = opened.open_read(path).unwrap();
    // The size is reported when the file is opened; the file then loses its tail.
    std::fs::write(camera.local(path), [7; 1_000]).unwrap();

    let mut content = Vec::new();
    let error = reader.read_to_end(&mut content).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
}
