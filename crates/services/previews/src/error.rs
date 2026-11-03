// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Errors of the [`PreviewDecoder`](crate::PreviewDecoder) port.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// A full decode failed (PREVIEWS-001); every variant names the file.
#[derive(Debug, Error)]
pub enum DecodeError {
    /// The file cannot be opened or read (PREVIEWS-001.8).
    #[error("cannot read {}: {source}", path.display())]
    Io {
        /// The file that was to be decoded.
        path: PathBuf,
        /// Why it cannot be read.
        source: io::Error,
    },
    /// The decoder does not support the file's format, such as GPR
    /// (PREVIEWS-001.6); CORE-004 then shows the embedded preview.
    #[error("cannot decode {}: the format is not supported", path.display())]
    UnsupportedFormat {
        /// The file that was to be decoded.
        path: PathBuf,
    },
    /// The file is truncated or its data is damaged (PREVIEWS-001.7).
    #[error("cannot decode {}: the file is corrupt: {message}", path.display())]
    Corrupt {
        /// The file that was to be decoded.
        path: PathBuf,
        /// The decoder's message.
        message: String,
    },
    /// Any other decoder failure, such as running out of memory.
    #[error("cannot decode {}: {message}", path.display())]
    Failed {
        /// The file that was to be decoded.
        path: PathBuf,
        /// The decoder's message.
        message: String,
    },
}

/// The sides and samples given for a [`DecodedImage`](crate::DecodedImage) do not fit
/// together.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("{samples} samples do not fit an image of {width} × {height} pixels with 3 samples each")]
pub struct ImageLayoutError {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) samples: usize,
}
