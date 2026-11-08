// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: the LibRaw decoder passes the conformance suite of the
//! `PreviewDecoder` port with the files of `testdata.lock` (PREVIEWS-001).
//!
//! A reference decode of the X-Trans RAF takes about 20 s (ADR 0028), and the suite
//! needs two of them. One runs on a background thread beside the suite's first, and
//! it is shared: the suite gets it as its second reference decode, and the version
//! test of the RAF checks it afterwards. Each run still decodes every file.

#![allow(clippy::unwrap_used, reason = "a test fails on any decode error")]

mod reference;
mod testdata;

use std::cell::{Cell, OnceCell, RefCell};
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};

use adapter_libraw::LibRawDecoder;
use previews::testing::{PreviewDecoderUnderTest, check_preview_decoder};
use previews::{DecodeError, DecodeMode, DecodedImage, PreviewDecoder};

/// The LibRaw decoder, except that its second reference decode of `shared_path` is
/// the one a background thread ran beside the first.
struct SharingDecoder {
    decoder: LibRawDecoder,
    shared_path: PathBuf,
    background: RefCell<Option<JoinHandle<Result<DecodedImage, DecodeError>>>>,
    shared: OnceCell<Result<DecodedImage, String>>,
    decoded_once: Cell<bool>,
}

impl SharingDecoder {
    /// Starts the reference decode of `path` on a background thread.
    fn start(path: PathBuf) -> Self {
        let background = thread::spawn({
            let path = path.clone();
            move || LibRawDecoder::new().decode_full(&path, DecodeMode::Reference)
        });
        SharingDecoder {
            decoder: LibRawDecoder::new(),
            shared_path: path,
            background: RefCell::new(Some(background)),
            shared: OnceCell::new(),
            decoded_once: Cell::new(false),
        }
    }

    /// The background decode, waiting for it on the first call.
    fn shared(&self) -> &Result<DecodedImage, String> {
        self.shared.get_or_init(|| {
            let background = self.background.take().unwrap();
            background
                .join()
                .unwrap()
                .map_err(|error| error.to_string())
        })
    }
}

impl PreviewDecoder for SharingDecoder {
    fn version(&self) -> String {
        self.decoder.version()
    }

    fn decode_full(&self, path: &Path, mode: DecodeMode) -> Result<DecodedImage, DecodeError> {
        let shareable = mode == DecodeMode::Reference && path == self.shared_path;
        if shareable && self.decoded_once.replace(true) {
            return self
                .shared()
                .clone()
                .map_err(|message| DecodeError::Failed {
                    path: path.to_owned(),
                    message,
                });
        }
        self.decoder.decode_full(path, mode)
    }
}

struct LibRawUnderTest {
    decoder: SharingDecoder,
    truncated: PathBuf,
}

impl PreviewDecoderUnderTest for LibRawUnderTest {
    fn decoder(&self) -> &dyn PreviewDecoder {
        &self.decoder
    }

    fn supported_file(&self) -> PathBuf {
        testdata::path(reference::XTRANS_RAF.relative)
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
/// spec: PREVIEWS-001.4
/// spec: PREVIEWS-001.6
/// spec: PREVIEWS-001.7
/// spec: PREVIEWS-001.8
#[test]
fn conforms_to_the_port_and_decodes_the_x_trans_raf_to_its_reference() {
    let raf = testdata::path(reference::XTRANS_RAF.relative);
    let under_test = LibRawUnderTest {
        decoder: SharingDecoder::start(raf),
        truncated: testdata::truncated_copy(reference::XTRANS_RAF.relative),
    };

    let failures = check_preview_decoder(&under_test);

    assert!(failures.is_empty(), "{failures:#?}");
    let image = under_test.decoder.shared().as_ref().unwrap();
    reference::assert_matches(&reference::XTRANS_RAF, image);
}
