// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The camera model read from files (IMPORT-021).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use std::path::Path;

use domain::Timestamp;
use files::testing::MemoryFileSystem;
use metadata::testing::{ByteOrder, MODEL, Tiff, jpeg, raf};
use metadata::{camera_model_from_bytes, read_camera_model};

fn tiff(order: ByteOrder) -> Vec<u8> {
    Tiff::new(order).with_ifd0_text(MODEL, "X-T5").to_bytes()
}

fn read(name: &str, content: &[u8]) -> Option<String> {
    let file_system = MemoryFileSystem::new();
    let path = Path::new("/card").join(name);
    file_system.add_file(&path, content, Timestamp::UNIX_EPOCH);
    read_camera_model(&file_system, &path).unwrap()
}

/// spec: IMPORT-021.8
#[test]
fn jpeg_raw_and_raf_give_the_model() {
    for order in [ByteOrder::LittleEndian, ByteOrder::BigEndian] {
        let raw = tiff(order);
        assert_eq!(read("a.NEF", &raw).as_deref(), Some("X-T5"));
        assert_eq!(read("a.JPG", &jpeg(&raw)).as_deref(), Some("X-T5"));
        assert_eq!(read("a.RAF", &raf(&jpeg(&raw))).as_deref(), Some("X-T5"));
    }
}

/// spec: IMPORT-021.8
#[test]
fn a_value_ends_at_its_first_nul() {
    let raw = Tiff::new(ByteOrder::LittleEndian)
        .with_ifd0_text(MODEL, "X-T5\0junk")
        .to_bytes();

    assert_eq!(camera_model_from_bytes(&raw).as_deref(), Some("X-T5"));
}

/// spec: IMPORT-021.8
#[test]
fn files_without_a_model_give_none() {
    let no_model = Tiff::new(ByteOrder::LittleEndian).to_bytes();
    let mut damaged = tiff(ByteOrder::LittleEndian);
    damaged.truncate(10);
    let empty_model = Tiff::new(ByteOrder::LittleEndian)
        .with_ifd0_text(MODEL, "")
        .to_bytes();

    assert_eq!(read("a.NEF", &no_model), None);
    assert_eq!(read("a.JPG", &jpeg(&no_model)), None);
    assert_eq!(read("a.JPG", b"\xFF\xD8\xFF\xD9"), None);
    assert_eq!(read("a.NEF", &damaged), None);
    assert_eq!(read("a.NEF", &empty_model), None);
    assert_eq!(read("a.NEF", b""), None);
    assert_eq!(read("a.CR3", &tiff(ByteOrder::LittleEndian)), None);
    assert_eq!(read("a.JPG", b"not an image"), None);
}

/// spec: IMPORT-021.8
#[test]
fn a_model_longer_than_the_read_limit_is_cut() {
    let long = "m".repeat(200);
    let raw = Tiff::new(ByteOrder::LittleEndian)
        .with_ifd0_text(MODEL, &long)
        .to_bytes();

    let model = camera_model_from_bytes(&raw).unwrap();

    assert!(long.starts_with(&model) && !model.is_empty());
}
