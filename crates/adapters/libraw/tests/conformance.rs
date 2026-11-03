// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: the LibRaw decoder passes the conformance suite of the
//! `PreviewDecoder` port with the files of `testdata.lock` (PREVIEWS-001).

#![allow(clippy::unwrap_used, reason = "a test fails on any decode error")]

mod testdata;

use std::path::PathBuf;

use adapter_libraw::LibRawDecoder;
use previews::PreviewDecoder;
use previews::testing::{PreviewDecoderUnderTest, check_preview_decoder};

struct LibRawUnderTest {
    decoder: LibRawDecoder,
    truncated: PathBuf,
}

impl PreviewDecoderUnderTest for LibRawUnderTest {
    fn decoder(&self) -> &dyn PreviewDecoder {
        &self.decoder
    }

    fn supported_file(&self) -> PathBuf {
        testdata::path(testdata::XTRANS_RAF)
    }

    fn unsupported_file(&self) -> PathBuf {
        testdata::path(testdata::GPR)
    }

    fn truncated_file(&self) -> PathBuf {
        self.truncated.clone()
    }

    fn missing_path(&self) -> PathBuf {
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("missing.RAF")
    }
}

/// spec: PREVIEWS-001.1
/// spec: PREVIEWS-001.2
/// spec: PREVIEWS-001.3
/// spec: PREVIEWS-001.6
/// spec: PREVIEWS-001.7
/// spec: PREVIEWS-001.8
#[test]
fn conforms_to_the_port() {
    let under_test = LibRawUnderTest {
        decoder: LibRawDecoder::new(),
        truncated: testdata::truncated_copy(testdata::XTRANS_RAF),
    };

    let failures = check_preview_decoder(&under_test);

    assert!(failures.is_empty(), "{failures:#?}");
}
