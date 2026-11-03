// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The [`PreviewDecoder`] port.

use std::path::Path;

use crate::{DecodeError, DecodedImage};

/// How many threads a full decode uses (PREVIEWS-001, ADR 0028 §3).
///
/// Both modes otherwise decode alike: 16-bit sRGB, the camera's white balance, no
/// automatic brightening and the decoder's default demosaicing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeMode {
    /// One thread, so that the output is bit-identical between runs; the reference
    /// decode of the version test.
    Reference,
    /// All threads, for 100% zoom and comparison; X-Trans files may differ in a few
    /// pixel values between runs.
    Interactive,
}

/// Port through which Framehive decodes an original at full resolution
/// (PREVIEWS-001, ADR 0012); implemented by `adapter-libraw`.
///
/// Every implementation passes [`testing::check_preview_decoder`](crate::testing::check_preview_decoder).
pub trait PreviewDecoder {
    /// The decoder's name and version, such as `LibRaw 0.22.1`; the reference hashes
    /// are stored for one version (PREVIEWS-001.5).
    fn version(&self) -> String;

    /// Decodes the RAW at `path` at full resolution in `mode`.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::Io`] if the file cannot be read,
    /// [`DecodeError::UnsupportedFormat`] if the decoder does not support its format,
    /// [`DecodeError::Corrupt`] if it is truncated or damaged, and
    /// [`DecodeError::Failed`] for any other decoder failure.
    fn decode_full(&self, path: &Path, mode: DecodeMode) -> Result<DecodedImage, DecodeError>;
}
