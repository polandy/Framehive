// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `blake3` adapter of Framehive: it implements the [`files::Hasher`] port with
//! BLAKE3, the checksum of Framehive (architecture §8, ADR 0020 §5). Only apps depend
//! on it. See `README.md`.
//!
//! ```
//! use adapter_blake3::Blake3Hasher;
//! use files::Hasher;
//!
//! let mut state = Blake3Hasher.start();
//! state.update(b"abc");
//! let checksum = state.finish();
//! assert_eq!(checksum.algorithm(), "blake3");
//! assert_eq!(checksum.digest().len(), 32);
//! ```

#![forbid(unsafe_code)]

use domain::Checksum;
use files::{HashState, Hasher};

/// The [`Hasher`] that computes BLAKE3 checksums with the default 32-byte output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Blake3Hasher;

impl Hasher for Blake3Hasher {
    fn start(&self) -> Box<dyn HashState> {
        Box::new(Blake3State(blake3::Hasher::new()))
    }
}

/// A BLAKE3 hash in progress.
struct Blake3State(blake3::Hasher);

impl HashState for Blake3State {
    fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    fn finish(self: Box<Self>) -> Checksum {
        let digest = self.0.finalize().as_bytes().to_vec();
        // The id is valid and the digest has 32 bytes, so `new` cannot fail.
        Checksum::new(Checksum::BLAKE3, digest).unwrap_or_else(|error| unreachable!("{error}"))
    }
}
