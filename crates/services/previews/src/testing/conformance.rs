// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance suite of the [`PreviewDecoder`] port (PREVIEWS-001).

use std::io;
use std::path::{Path, PathBuf};

use crate::{DecodeError, DecodeMode, DecodedImage, PreviewDecoder};

/// A preview decoder and the files it is checked with, for [`check_preview_decoder`].
pub trait PreviewDecoderUnderTest {
    /// The implementation under test.
    fn decoder(&self) -> &dyn PreviewDecoder;

    /// A RAW file the decoder supports; for a RAW decoder, an X-Trans RAF, because
    /// its demosaicing is the one that can differ between runs (ADR 0028).
    fn supported_file(&self) -> PathBuf;

    /// A file in a format the decoder does not support, such as GPR.
    fn unsupported_file(&self) -> PathBuf;

    /// A supported RAW file truncated to half its length.
    fn truncated_file(&self) -> PathBuf;

    /// A path where no file exists.
    fn missing_path(&self) -> PathBuf;
}

/// Runs the conformance suite of the [`PreviewDecoder`] port and returns one message
/// per failed check, or nothing if the implementation conforms.
#[must_use]
pub fn check_preview_decoder(under_test: &dyn PreviewDecoderUnderTest) -> Vec<String> {
    let mut failures = Vec::new();
    check_version(under_test, &mut failures);
    check_decoded_modes(under_test, &mut failures);
    check_unsupported(under_test, &mut failures);
    check_truncated(under_test, &mut failures);
    check_missing(under_test, &mut failures);
    failures
}

fn check_version(under_test: &dyn PreviewDecoderUnderTest, failures: &mut Vec<String>) {
    if under_test.decoder().version().trim().is_empty() {
        failures.push("version: the decoder reports an empty version".to_owned());
    }
}

/// Decodes the supported file twice in reference mode and once in interactive mode,
/// because a reference decode of an X-Trans RAF takes about 20 s (ADR 0028).
fn check_decoded_modes(under_test: &dyn PreviewDecoderUnderTest, failures: &mut Vec<String>) {
    let path = under_test.supported_file();
    let mut decode = |mode| decoded(under_test, &path, mode, failures);
    let first = decode(DecodeMode::Reference);
    let second = decode(DecodeMode::Reference);
    let interactive = decode(DecodeMode::Interactive);
    let (Some(first), Some(second), Some(interactive)) = (first, second, interactive) else {
        return;
    };
    if first != second {
        failures.push(format!(
            "reference_deterministic: two reference decodes of {} differ",
            path.display()
        ));
    }
    if size(&interactive) != size(&first) {
        failures.push(format!(
            "interactive_size: {} decodes to {:?} in interactive mode and {:?} in reference mode",
            path.display(),
            size(&interactive),
            size(&first)
        ));
    }
}

fn decoded(
    under_test: &dyn PreviewDecoderUnderTest,
    path: &Path,
    mode: DecodeMode,
    failures: &mut Vec<String>,
) -> Option<DecodedImage> {
    match under_test.decoder().decode_full(path, mode) {
        Ok(image) => Some(image),
        Err(error) => {
            failures.push(format!("decode: decoding in {mode:?} mode fails: {error}"));
            None
        }
    }
}

fn size(image: &DecodedImage) -> (u32, u32) {
    (image.width(), image.height())
}

fn check_unsupported(under_test: &dyn PreviewDecoderUnderTest, failures: &mut Vec<String>) {
    let path = under_test.unsupported_file();
    match under_test
        .decoder()
        .decode_full(&path, DecodeMode::Interactive)
    {
        Err(DecodeError::UnsupportedFormat { path: named }) if named == path => {}
        other => failures.push(format!(
            "unsupported: {} gives {}, expected UnsupportedFormat naming it",
            path.display(),
            outcome(&other)
        )),
    }
}

fn check_truncated(under_test: &dyn PreviewDecoderUnderTest, failures: &mut Vec<String>) {
    let path = under_test.truncated_file();
    match under_test
        .decoder()
        .decode_full(&path, DecodeMode::Interactive)
    {
        Err(DecodeError::Corrupt { path: named, .. }) if named == path => {}
        other => failures.push(format!(
            "truncated: {} gives {}, expected Corrupt naming it",
            path.display(),
            outcome(&other)
        )),
    }
}

fn check_missing(under_test: &dyn PreviewDecoderUnderTest, failures: &mut Vec<String>) {
    let path = under_test.missing_path();
    match under_test
        .decoder()
        .decode_full(&path, DecodeMode::Interactive)
    {
        Err(DecodeError::Io {
            path: named,
            source,
        }) if named == path && source.kind() == io::ErrorKind::NotFound => {}
        other => failures.push(format!(
            "missing: {} gives {}, expected Io naming it with \"not found\"",
            path.display(),
            outcome(&other)
        )),
    }
}

fn outcome(result: &Result<DecodedImage, DecodeError>) -> String {
    match result {
        Ok(image) => format!("an image of {:?}", size(image)),
        Err(error) => format!("\"{error}\""),
    }
}
