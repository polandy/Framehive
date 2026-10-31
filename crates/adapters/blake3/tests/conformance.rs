// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: [`Blake3Hasher`] passes the conformance suite of the
//! `files::Hasher` port (CLAUDE.md, "Adding things").

use adapter_blake3::Blake3Hasher;
use domain::Checksum;
use files::testing::{HasherUnderTest, check_hasher};

/// Input of the official BLAKE3 test vectors: byte `i` is `i % 251`.
fn test_vector_input(length: usize) -> Vec<u8> {
    (0..=250).cycle().take(length).collect()
}

#[test]
fn conforms_to_the_hasher_port() {
    let one_kibibyte_and_one = test_vector_input(1_025);
    let known_checksums: [(&[u8], &str); 3] = [
        (
            b"",
            "blake3:af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
        ),
        (
            b"abc",
            "blake3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85",
        ),
        (
            &one_kibibyte_and_one,
            "blake3:d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444",
        ),
    ];
    let failures = check_hasher(&HasherUnderTest {
        hasher: &Blake3Hasher,
        algorithm: Checksum::BLAKE3,
        known_checksums: &known_checksums,
    });
    assert!(failures.is_empty(), "{failures:#?}");
}
