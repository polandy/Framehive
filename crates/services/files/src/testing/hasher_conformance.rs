// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`Hasher`] port.

use domain::Checksum;

use crate::Hasher;

/// What the [`Hasher`] conformance suite runs against.
pub struct HasherUnderTest<'a> {
    /// The implementation under test.
    pub hasher: &'a dyn Hasher,
    /// The algorithm id its checksums must carry, such as `blake3`.
    pub algorithm: &'a str,
    /// Inputs and their checksums in written form, from the algorithm's reference.
    pub known_checksums: &'a [(&'a [u8], &'a str)],
}

/// Runs the conformance suite of the [`Hasher`] port and returns one message per
/// failed check, or nothing if the implementation conforms.
#[must_use]
pub fn check_hasher(under_test: &HasherUnderTest<'_>) -> Vec<String> {
    let mut failures = Vec::new();
    check_known_checksums(under_test, &mut failures);
    check_algorithm(under_test, &mut failures);
    check_pieces(under_test, &mut failures);
    check_independent_states(under_test, &mut failures);
    check_different_content(under_test, &mut failures);
    failures
}

fn checksum_of(hasher: &dyn Hasher, pieces: &[&[u8]]) -> Checksum {
    let mut state = hasher.start();
    for piece in pieces {
        state.update(piece);
    }
    state.finish()
}

fn sample() -> Vec<u8> {
    (0..=255).cycle().take(3_000).collect()
}

fn check_known_checksums(under_test: &HasherUnderTest<'_>, failures: &mut Vec<String>) {
    if under_test.known_checksums.is_empty() {
        failures
            .push("known_checksums: the suite needs at least one reference checksum".to_owned());
    }
    for (input, expected) in under_test.known_checksums {
        let written = checksum_of(under_test.hasher, &[input]).to_string();
        if written != *expected {
            failures.push(format!(
                "known_checksums: {} bytes give {written}, expected {expected}",
                input.len()
            ));
        }
    }
}

fn check_algorithm(under_test: &HasherUnderTest<'_>, failures: &mut Vec<String>) {
    for input in [&b""[..], &sample()] {
        let checksum = checksum_of(under_test.hasher, &[input]);
        if checksum.algorithm() != under_test.algorithm {
            failures.push(format!(
                "algorithm: checksum {checksum} does not carry the id {}",
                under_test.algorithm
            ));
        }
    }
}

fn check_pieces(under_test: &HasherUnderTest<'_>, failures: &mut Vec<String>) {
    let content = sample();
    let whole = checksum_of(under_test.hasher, &[&content]);
    for split in [0, 1, 64, 1_024, 1_025, content.len()] {
        let (first, second) = content.split_at(split);
        let pieces = checksum_of(under_test.hasher, &[first, &[], second]);
        if pieces != whole {
            failures.push(format!(
                "pieces: splitting at byte {split} gives {pieces}, expected {whole}"
            ));
        }
    }
}

fn check_independent_states(under_test: &HasherUnderTest<'_>, failures: &mut Vec<String>) {
    let mut first = under_test.hasher.start();
    let mut second = under_test.hasher.start();
    first.update(b"first");
    second.update(b"second");
    let (first, second) = (first.finish(), second.finish());
    if first != checksum_of(under_test.hasher, &[b"first"])
        || second != checksum_of(under_test.hasher, &[b"second"])
    {
        failures.push("independent_states: two hashes in progress influence each other".to_owned());
    }
}

fn check_different_content(under_test: &HasherUnderTest<'_>, failures: &mut Vec<String>) {
    let content = sample();
    let mut changed = content.clone();
    if let Some(last) = changed.last_mut() {
        *last ^= 1;
    }
    let original = checksum_of(under_test.hasher, &[&content]);
    if original == checksum_of(under_test.hasher, &[&changed]) {
        failures.push("different_content: changing one bit keeps the checksum".to_owned());
    }
    if checksum_of(under_test.hasher, &[b""]) == checksum_of(under_test.hasher, &[b"\0"]) {
        failures.push(
            "different_content: no bytes and one zero byte give the same checksum".to_owned(),
        );
    }
}
