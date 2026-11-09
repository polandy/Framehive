// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Capture time and camera model of real camera files from `testdata.lock`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests may panic on bad fixtures or missing test data"
)]

use std::env;
use std::path::{Path, PathBuf};

use domain::{CaptureTime, LocalDateTime, Timestamp};
use files::testing::MemoryFileSystem;
use metadata::{read_camera_model, read_capture_time};

/// The Canon EOS 350D set: a Lightroom DNG and the camera JPEG of one shot.
const EOS_350D: [&str; 2] = [
    "dng/canon-eos-350d/eos-350d-cosmos.dng",
    "dng/canon-eos-350d/eos-350d-cosmos.JPG",
];

/// The test data directory, as `cargo xtask testdata fetch` fills it.
fn testdata_directory() -> PathBuf {
    env::var_os("FRAMEHIVE_TESTDATA_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.testdata"),
        PathBuf::from,
    )
}

/// A memory file system holding the test data file `relative` at `/card/<name>`.
fn card_with(relative: &str) -> (MemoryFileSystem, PathBuf) {
    let source = testdata_directory().join(relative);
    let content = std::fs::read(&source).unwrap_or_else(|error| {
        panic!("test data {relative} is missing ({error}); run `cargo xtask testdata fetch`")
    });
    let path = Path::new("/card").join(Path::new(relative).file_name().unwrap());
    let file_system = MemoryFileSystem::new();
    file_system.add_file(&path, &content, Timestamp::UNIX_EPOCH);
    (file_system, path)
}

/// spec: CORE-005.13
#[test]
fn eos_350d_dng_and_jpeg_give_the_capture_time() {
    let taken = CaptureTime::new(LocalDateTime::new(2016, 10, 12, 14, 1, 18).unwrap());
    for relative in EOS_350D {
        let (file_system, path) = card_with(relative);

        assert_eq!(
            read_capture_time(&file_system, &path).unwrap(),
            taken,
            "{relative}"
        );
    }
}

/// spec: IMPORT-021.9
#[test]
fn eos_350d_dng_and_jpeg_give_the_camera_model() {
    for relative in EOS_350D {
        let (file_system, path) = card_with(relative);

        let model = read_camera_model(&file_system, &path).unwrap();

        assert_eq!(
            model.as_deref(),
            Some("Canon EOS 350D DIGITAL"),
            "{relative}"
        );
    }
}
