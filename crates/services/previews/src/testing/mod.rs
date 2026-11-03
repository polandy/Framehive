// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Test doubles and the conformance suite of the
//! [`PreviewDecoder`](crate::PreviewDecoder) port (PREVIEWS-001).
//!
//! `adapter-libraw` runs [`check_preview_decoder`] against the files in
//! `testdata.lock`.

mod conformance;
mod memory;

pub use conformance::{PreviewDecoderUnderTest, check_preview_decoder};
pub use memory::MemoryPreviewDecoder;
