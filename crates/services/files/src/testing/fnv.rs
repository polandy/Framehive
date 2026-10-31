// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A small, non-cryptographic [`Hasher`] for tests.

use domain::Checksum;

use crate::{HashState, Hasher};

const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// A [`Hasher`] with 64-bit FNV-1a, for tests that need no real checksum.
///
/// Its checksums carry the algorithm id `fnv`, so they never pass for BLAKE3
/// checksums.
///
/// ```
/// use files::Hasher;
/// use files::testing::FnvHasher;
///
/// let mut hash = FnvHasher.start();
/// hash.update(b"a");
/// assert_eq!(hash.finish().to_string(), "fnv:af63dc4c8601ec8c");
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct FnvHasher;

impl FnvHasher {
    /// The algorithm id of its checksums.
    pub const ALGORITHM: &'static str = "fnv";
}

impl Hasher for FnvHasher {
    fn start(&self) -> Box<dyn HashState> {
        Box::new(FnvState(OFFSET_BASIS))
    }
}

struct FnvState(u64);

impl HashState for FnvState {
    fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(PRIME);
        }
    }

    fn finish(self: Box<Self>) -> Checksum {
        // The id is valid and the digest has eight bytes, so `new` cannot fail.
        Checksum::new(FnvHasher::ALGORITHM, self.0.to_be_bytes().to_vec())
            .unwrap_or_else(|error| unreachable!("{error}"))
    }
}
