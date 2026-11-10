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

use domain::{CaptureTime, LocalDateTime, Timestamp, UtcOffset};
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

/// A file of the owner's X-T2, X-T5 and two EOS 7D sets with the values it gives.
struct Expected {
    relative: &'static str,
    taken: CaptureTime,
    model: &'static str,
}

/// The RAW and the camera JPEG of one shot from each of the four sets.
fn other_cameras() -> [Expected; 8] {
    let at = |year, month, day, hour, minute, second| {
        CaptureTime::new(LocalDateTime::new(year, month, day, hour, minute, second).unwrap())
    };
    let x_t2 = at(2023, 10, 2, 12, 5, 23);
    let x_t5 = at(2024, 9, 28, 13, 25, 29).with_offset(UtcOffset::from_seconds(7_200).unwrap());
    // Lightroom shifted the DNG by six hours; the camera JPEG keeps the camera's clock.
    let eos_7d_dng = at(2015, 8, 10, 10, 41, 56);
    let eos_7d_jpeg = at(2015, 8, 10, 4, 41, 56)
        .with_subsec_nanoseconds(330_000_000)
        .unwrap();
    let eos_7d_adobe_rgb = at(2014, 10, 16, 15, 58, 5)
        .with_subsec_nanoseconds(0)
        .unwrap();
    [
        ("fujifilm/x-t2/x-t2-stone-villa.RAF", x_t2, "X-T2"),
        ("fujifilm/x-t2/x-t2-stone-villa.JPG", x_t2, "X-T2"),
        ("fujifilm/x-t5/x-t5-mountain-lake.raf", x_t5, "X-T5"),
        ("fujifilm/x-t5/x-t5-mountain-lake.jpg", x_t5, "X-T5"),
        (
            "canon/eos-7d/eos-7d-garden-house.dng",
            eos_7d_dng,
            "Canon EOS 7D",
        ),
        (
            "canon/eos-7d/eos-7d-garden-house.JPG",
            eos_7d_jpeg,
            "Canon EOS 7D",
        ),
        (
            "canon/eos-7d-v2/_MG_1309.CR2",
            eos_7d_adobe_rgb,
            "Canon EOS 7D",
        ),
        (
            "canon/eos-7d-v2/_MG_1309.JPG",
            eos_7d_adobe_rgb,
            "Canon EOS 7D",
        ),
    ]
    .map(|(relative, taken, model)| Expected {
        relative,
        taken,
        model,
    })
}

/// spec: CORE-005.14
#[test]
fn x_t2_x_t5_and_eos_7d_files_give_the_capture_time() {
    for expected in other_cameras() {
        let (file_system, path) = card_with(expected.relative);

        assert_eq!(
            read_capture_time(&file_system, &path).unwrap(),
            expected.taken,
            "{}",
            expected.relative
        );
    }
}

/// spec: IMPORT-021.10
#[test]
fn x_t2_x_t5_and_eos_7d_files_give_the_camera_model() {
    for expected in other_cameras() {
        let (file_system, path) = card_with(expected.relative);

        let model = read_camera_model(&file_system, &path).unwrap();

        assert_eq!(
            model.as_deref(),
            Some(expected.model),
            "{}",
            expected.relative
        );
    }
}
