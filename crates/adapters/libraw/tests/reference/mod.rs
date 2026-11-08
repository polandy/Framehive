// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The reference values of the version test of ADR 0012 §3 and their comparison with
//! a decoded image (PREVIEWS-001).

#![allow(dead_code, reason = "each test binary uses only some of the helpers")]

use adapter_libraw::LibRawDecoder;
use previews::{DecodedImage, PreviewDecoder};

/// The decoder version the reference values were stored for.
pub(crate) const PINNED_VERSION: &str = "LibRaw 0.22.1";

/// Whether the pixel hashes were measured on this platform; elsewhere only the size is
/// compared until hashes for it are stored (PREVIEWS-001).
const HASHES_MEASURED: bool = cfg!(all(target_os = "linux", target_arch = "x86_64"));

/// The stored size and pixel hash of a file in `testdata.lock`.
pub(crate) struct Reference {
    /// The file's path in the test data directory.
    pub(crate) relative: &'static str,
    /// The width in pixels.
    pub(crate) width: u32,
    /// The height in pixels.
    pub(crate) height: u32,
    /// The BLAKE3 hash of the samples on Linux `x86_64`.
    pub(crate) hash: &'static str,
}

/// The X-T5 lossy compressed RAF, an X-Trans file (ADR 0028); the conformance test
/// checks it with a reference decode it shares with the port's determinism check.
pub(crate) const XTRANS_RAF: Reference = Reference {
    relative: "raw.pixls.us/Fujifilm/X-T5/DSCF0022.RAF",
    width: 7752,
    height: 5178,
    hash: "33570674a0c7e3cad53eaffe4b653ad5dec78575a0e2b27b066d9ebbe202035e",
};

/// Asserts that `image`, decoded from `reference.relative` in reference mode, has the
/// stored size and pixel hash of the pinned decoder.
pub(crate) fn assert_matches(reference: &Reference, image: &DecodedImage) {
    assert_eq!(
        LibRawDecoder::new().version(),
        PINNED_VERSION,
        "the reference values are for {PINNED_VERSION}"
    );
    assert_eq!(
        (image.width(), image.height()),
        (reference.width, reference.height),
        "{}",
        reference.relative
    );
    if HASHES_MEASURED {
        let actual = blake3::hash(&image.sample_bytes()).to_hex();
        assert_eq!(
            actual.as_str(),
            reference.hash,
            "pixel hash of {}",
            reference.relative
        );
    }
}
