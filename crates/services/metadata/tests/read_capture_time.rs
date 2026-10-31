// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The capture time of files read through the `FileSystem` port (CORE-005).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod common;

use std::path::Path;

use common::CountingFileSystem;
use domain::{CaptureTime, LocalDateTime};
use files::testing::MemoryFileSystem;
use metadata::testing::{ByteOrder, DATE_TIME_ORIGINAL, Tiff, jpeg, raf};
use metadata::{CaptureTimeError, NoCaptureTime, read_capture_time};

const MEGABYTE: u64 = 1_000_000;

fn taken() -> CaptureTime {
    CaptureTime::new(LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap())
}

fn tiff() -> Vec<u8> {
    Tiff::new(ByteOrder::LittleEndian)
        .with_exif_text(DATE_TIME_ORIGINAL, "2026:10:17 14:05:09")
        .to_bytes()
}

/// spec: CORE-005.10
#[test]
fn formats_not_in_the_table_are_not_opened() {
    let file_system = CountingFileSystem::new();
    for name in ["/card/clip.MOV", "/card/IMG_0001.CR3", "/card/photo.HIF"] {
        // TIFF content, so only the extension keeps the file from being read.
        file_system.add_file(Path::new(name), &tiff());

        let reason = read_capture_time(&file_system, Path::new(name)).unwrap_err();

        assert_eq!(reason.to_string(), "format not read", "{name}");
    }
    assert_eq!(file_system.opened(), 0);
}

/// spec: CORE-005.12
#[test]
fn large_raw_is_read_only_near_its_start() {
    let mut raw = tiff();
    raw.resize(50 * 1024 * 1024, 0xA5);
    let file_system = CountingFileSystem::new();
    file_system.add_file(Path::new("/card/DSC_0001.NEF"), &raw);

    let time = read_capture_time(&file_system, Path::new("/card/DSC_0001.NEF"));

    assert_eq!(time.unwrap(), taken());
    assert!(
        file_system.bytes_read() <= MEGABYTE,
        "{} bytes read",
        file_system.bytes_read()
    );
}

/// spec: CORE-005.12
#[test]
fn jpeg_of_fill_bytes_is_not_read_in_full() {
    // Fill bytes before a marker are skipped one at a time, so only the read budget
    // stops the walk.
    let mut content = vec![0xFF, 0xD8];
    content.resize(2 * 1024 * 1024, 0xFF);
    let file_system = CountingFileSystem::new();
    file_system.add_file(Path::new("/card/fill.JPG"), &content);

    let reason = read_capture_time(&file_system, Path::new("/card/fill.JPG")).unwrap_err();

    assert!(reason.to_string().starts_with("damaged"), "{reason}");
    assert!(
        file_system.bytes_read() <= MEGABYTE,
        "{} bytes read",
        file_system.bytes_read()
    );
}

#[test]
fn every_container_is_read_whatever_the_case_of_its_extension() {
    let file_system = MemoryFileSystem::new();
    let files = [
        ("/card/a.jpg", jpeg(&tiff())),
        ("/card/b.MPO", jpeg(&tiff())),
        ("/card/c.Dng", tiff()),
        ("/card/d.RAF", raf(&jpeg(&tiff()))),
    ];
    for (name, content) in &files {
        file_system.add_file(Path::new(name), content, domain::Timestamp::UNIX_EPOCH);
    }

    for (name, _) in &files {
        assert_eq!(
            read_capture_time(&file_system, Path::new(name)).unwrap(),
            taken(),
            "{name}"
        );
    }
}

#[test]
fn content_that_does_not_match_its_extension_is_not_read() {
    let file_system = MemoryFileSystem::new();
    let path = Path::new("/card/a.JPG");
    file_system.add_file(path, b"\x89PNG\r\n\x1a\n", domain::Timestamp::UNIX_EPOCH);

    assert!(matches!(
        read_capture_time(&file_system, path),
        Err(CaptureTimeError::NoCaptureTime(
            NoCaptureTime::FormatNotRead
        ))
    ));
}

#[test]
fn missing_file_is_an_io_error_with_its_path() {
    let file_system = MemoryFileSystem::new();

    let error = read_capture_time(&file_system, Path::new("/card/gone.JPG")).unwrap_err();

    assert!(
        matches!(&error, CaptureTimeError::Io { path, .. } if path == Path::new("/card/gone.JPG")),
        "{error:?}"
    );
}
