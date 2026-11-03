// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The LibRaw adapter of Framehive: [`LibRawDecoder`] implements the
//! [`PreviewDecoder`](previews::PreviewDecoder) port of `previews` with LibRaw
//! (PREVIEWS-001, ADR 0012, ADR 0028). Only apps depend on it. See `README.md`.
//!
//! ```no_run
//! use std::path::Path;
//!
//! use adapter_libraw::LibRawDecoder;
//! use previews::{DecodeMode, PreviewDecoder};
//!
//! let image = LibRawDecoder::new()
//!     .decode_full(Path::new("/photos/DSCF0022.RAF"), DecodeMode::Interactive)?;
//! assert_eq!(image.samples().len(), image.width() as usize * image.height() as usize * 3);
//! # Ok::<(), previews::DecodeError>(())
//! ```

mod decoder;
mod ffi;

pub use decoder::LibRawDecoder;
