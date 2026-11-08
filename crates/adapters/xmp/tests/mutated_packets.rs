// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Damaged sidecars never crash the parser, and whatever it accepts it writes so that
//! it reads back the same (CORE-009, ADR 0037).
//!
//! A deterministic stand-in for fuzzing: the fixtures are truncated, have bytes
//! flipped, or have a markup character inserted, by a small PRNG with a fixed seed. A
//! failing case prints its number, so it can be replayed.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod common;

use std::panic::{AssertUnwindSafe, catch_unwind};

use common::{WRITE_TIME, fixture, parse_bytes, write_and_read, xmp};
use metadata::xmp::XmpNode;

const SEED: u64 = 0x5EED_CAFE_F00D_0009;
const CASES: usize = 3_000;
const FIXTURES: [&str; 10] = [
    "attributes.xmp",
    "elements.xmp",
    "nested.xmp",
    "parse-type.xmp",
    "split.xmp",
    "wrapped.xmp",
    "merge.xmp",
    "clashing-prefix.xmp",
    "rights.xmp",
    "darktable.xmp",
];
const MARKUP: &[u8] = b"<>/=\"'&;:# \n";

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
}

fn mutate(random: &mut Random, mut bytes: Vec<u8>) -> Vec<u8> {
    let at = random.below(bytes.len());
    match random.below(3) {
        0 => bytes.truncate(at),
        1 => bytes[at] ^= 1 << random.below(8),
        _ => bytes.insert(at, MARKUP[random.below(MARKUP.len())]),
    }
    bytes
}

#[test]
fn damaged_sidecars_never_crash_and_round_trip_when_accepted() {
    let originals: Vec<_> = FIXTURES.iter().map(|name| fixture(name)).collect();
    let mut random = Random(SEED);
    for case in 0..CASES {
        let original = originals[random.below(originals.len())].clone();
        let mutant = mutate(&mut random, original);
        let outcome = catch_unwind(AssertUnwindSafe(|| check(&mutant)));
        assert!(outcome.is_ok(), "case {case} panicked");
        assert!(outcome.unwrap(), "case {case} did not read back the same");
    }
}

/// True when `content` is refused, or written and read back to the same model.
fn check(content: &[u8]) -> bool {
    let Ok(packet) = parse_bytes("mutant.xmp", content) else {
        return true;
    };
    let mut expected = packet.properties().clone();
    expected.insert(xmp("MetadataDate"), XmpNode::text(WRITE_TIME));
    write_and_read(&packet).properties() == &expected
}
