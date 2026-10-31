// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Damaged files never crash the capture time reader nor make it read much
//! (CORE-005.9, CORE-005.12).
//!
//! A deterministic stand-in for fuzzing: valid files are built, then truncated, have
//! bytes flipped, have offsets and counts overwritten, or get a long tail, by a small
//! PRNG with a fixed seed. A failing case prints its number, so it can be replayed.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod common;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use common::CountingFileSystem;
use metadata::testing::{
    ByteOrder, DATE_TIME, DATE_TIME_ORIGINAL, OFFSET_TIME_ORIGINAL, SUB_SEC_TIME_ORIGINAL, Tiff,
    jpeg, raf,
};
use metadata::{capture_time_from_bytes, read_capture_time};

const SEED: u64 = 0x5EED_CAFE_F00D_0005;
const CASES: usize = 4_000;
const MEGABYTE: u64 = 1_000_000;
/// Values that make offsets and counts point at the edges of a file.
const EDGE_VALUES: [u32; 7] = [0, 1, 8, 0x7FFF_FFFF, 0xFFFF_FFFF, 0xFFFF, 0x0101_0101];

/// Xorshift64: small, fast and the same on every platform.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A number below `bound`, which must not be zero.
    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap()).unwrap()
    }

    fn byte(&mut self) -> u8 {
        self.next().to_le_bytes()[0]
    }
}

fn tiff(order: ByteOrder) -> Vec<u8> {
    Tiff::new(order)
        .with_ifd0_text(DATE_TIME, "2026:10:18 09:00:00")
        .with_exif_text(DATE_TIME_ORIGINAL, "2026:10:17 14:05:09")
        .with_exif_text(SUB_SEC_TIME_ORIGINAL, "42")
        .with_exif_text(OFFSET_TIME_ORIGINAL, "+02:00")
        .to_bytes()
}

/// Valid files of every container, with the extension that makes them read.
fn fixtures() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("/card/a.NEF", tiff(ByteOrder::LittleEndian)),
        ("/card/b.DNG", tiff(ByteOrder::BigEndian)),
        ("/card/c.JPG", jpeg(&tiff(ByteOrder::LittleEndian))),
        ("/card/d.RAF", raf(&jpeg(&tiff(ByteOrder::BigEndian)))),
    ]
}

/// Damages `content` in one of several ways.
fn mutate(random: &mut Random, mut content: Vec<u8>) -> Vec<u8> {
    if content.is_empty() {
        return content;
    }
    match random.below(5) {
        0 => content.truncate(random.below(content.len() + 1)),
        1 => {
            for _ in 0..=random.below(8) {
                let at = random.below(content.len());
                content[at] ^= random.byte() | 1;
            }
        }
        2 => {
            let at = random.below(content.len().saturating_sub(4) + 1);
            let value = EDGE_VALUES[random.below(EDGE_VALUES.len())];
            let bytes = if random.below(2) == 0 {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            };
            let end = (at + 4).min(content.len());
            content[at..end].copy_from_slice(&bytes[..end - at]);
        }
        3 => {
            // A 16-bit count, as of IFD entries or a JPEG segment length.
            let at = random.below(content.len().saturating_sub(2) + 1);
            let end = (at + 2).min(content.len());
            let count = random.byte();
            content[at..end].fill(count);
        }
        _ => {
            // A long tail the damaged offsets can point into.
            let tail = random.byte();
            content.resize(content.len() + 300 * 1024, tail);
        }
    }
    content
}

/// spec: CORE-005.9
/// spec: CORE-005.12
#[test]
fn mutated_files_give_a_reason_without_crash_and_bounded_reads() {
    let mut random = Random(SEED);
    let fixtures = fixtures();
    for case in 0..CASES {
        let (name, valid) = &fixtures[random.below(fixtures.len())];
        let mut content = mutate(&mut random, valid.clone());
        if random.below(4) == 0 {
            content = mutate(&mut random, content);
        }
        let file_system = CountingFileSystem::new();
        file_system.add_file(Path::new(name), &content);

        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let _ = read_capture_time(&file_system, Path::new(name));
            let _ = capture_time_from_bytes(&content);
        }));

        assert!(outcome.is_ok(), "case {case} ({name}) panicked");
        assert_eq!(file_system.opened(), 1, "case {case} ({name})");
        assert!(
            file_system.bytes_read() <= MEGABYTE,
            "case {case} ({name}) read {} bytes",
            file_system.bytes_read()
        );
    }
}

#[test]
fn the_unmutated_fixtures_have_a_capture_time() {
    for (name, content) in fixtures() {
        let file_system = CountingFileSystem::new();
        file_system.add_file(Path::new(name), &content);

        assert!(
            read_capture_time(&file_system, Path::new(name)).is_ok(),
            "{name}"
        );
    }
}
