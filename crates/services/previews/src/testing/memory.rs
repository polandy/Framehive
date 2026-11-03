// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A [`PreviewDecoder`] in memory.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::{DecodeError, DecodeMode, DecodedImage, PreviewDecoder};

/// A [`PreviewDecoder`] in memory: tests decide per path which image a decode returns
/// or how it fails. A path that was never inserted is not found.
///
/// ```
/// use std::path::Path;
///
/// use previews::testing::MemoryPreviewDecoder;
/// use previews::{DecodeError, DecodeMode, PreviewDecoder};
///
/// let mut decoder = MemoryPreviewDecoder::new();
/// decoder.insert_unsupported("/photos/GOPR0001.GPR");
///
/// let result = decoder.decode_full(Path::new("/photos/GOPR0001.GPR"), DecodeMode::Interactive);
/// assert!(matches!(result, Err(DecodeError::UnsupportedFormat { .. })));
/// ```
#[derive(Debug, Default)]
pub struct MemoryPreviewDecoder {
    files: HashMap<PathBuf, File>,
}

#[derive(Debug)]
enum File {
    Image(DecodedImage),
    Unsupported,
    Corrupt(String),
}

impl MemoryPreviewDecoder {
    /// The version this decoder reports.
    pub const VERSION: &str = "MemoryPreviewDecoder 1";

    /// A decoder that knows no files.
    #[must_use]
    pub fn new() -> Self {
        MemoryPreviewDecoder::default()
    }

    /// Makes decoding `path` return `image`, in both modes.
    pub fn insert_image(&mut self, path: impl Into<PathBuf>, image: DecodedImage) {
        self.files.insert(path.into(), File::Image(image));
    }

    /// Makes decoding `path` fail with [`DecodeError::UnsupportedFormat`].
    pub fn insert_unsupported(&mut self, path: impl Into<PathBuf>) {
        self.files.insert(path.into(), File::Unsupported);
    }

    /// Makes decoding `path` fail with [`DecodeError::Corrupt`] and `message`.
    pub fn insert_corrupt(&mut self, path: impl Into<PathBuf>, message: &str) {
        self.files
            .insert(path.into(), File::Corrupt(message.to_owned()));
    }
}

impl PreviewDecoder for MemoryPreviewDecoder {
    fn version(&self) -> String {
        Self::VERSION.to_owned()
    }

    fn decode_full(&self, path: &Path, _mode: DecodeMode) -> Result<DecodedImage, DecodeError> {
        let path_buf = path.to_path_buf();
        match self.files.get(path) {
            Some(File::Image(image)) => Ok(image.clone()),
            Some(File::Unsupported) => Err(DecodeError::UnsupportedFormat { path: path_buf }),
            Some(File::Corrupt(message)) => Err(DecodeError::Corrupt {
                path: path_buf,
                message: message.clone(),
            }),
            None => Err(DecodeError::Io {
                path: path_buf,
                source: io::Error::from(io::ErrorKind::NotFound),
            }),
        }
    }
}
