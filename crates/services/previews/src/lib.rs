// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The previews service of Framehive: the [`PreviewDecoder`] port, which decodes a
//! RAW at full resolution for 100% zoom and comparison (PREVIEWS-001, ADR 0012).
//!
//! The service never decodes itself; `adapter-libraw` implements the port.
//!
//! ```
//! use std::path::Path;
//!
//! use previews::testing::MemoryPreviewDecoder;
//! use previews::{DecodeMode, DecodedImage, PreviewDecoder};
//!
//! let mut decoder = MemoryPreviewDecoder::new();
//! decoder.insert_image("/photos/DSCF0021.RAF", DecodedImage::new(2, 1, vec![0; 6])?);
//!
//! let image = decoder.decode_full(Path::new("/photos/DSCF0021.RAF"), DecodeMode::Reference)?;
//! assert_eq!((image.width(), image.height()), (2, 1));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![forbid(unsafe_code)]

mod error;
mod image;
mod port;
pub mod testing;

pub use error::{DecodeError, ImageLayoutError};
pub use image::DecodedImage;
pub use port::{DecodeMode, PreviewDecoder};
