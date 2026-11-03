// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Decoded images keep the layout PREVIEWS-001 asks for, and the memory preview
//! decoder passes the conformance suite of the port.

use std::path::{Path, PathBuf};

use previews::testing::{MemoryPreviewDecoder, PreviewDecoderUnderTest, check_preview_decoder};
use previews::{DecodeError, DecodeMode, DecodedImage, ImageLayoutError, PreviewDecoder};

const RAF: &str = "/photos/DSCF0022.RAF";
const GPR: &str = "/photos/GOPR0001.GPR";
const TRUNCATED: &str = "/photos/DSCF0022-half.RAF";

fn image() -> Result<DecodedImage, ImageLayoutError> {
    DecodedImage::new(2, 1, vec![1, 2, 3, 0x0102, 0xfffe, 0])
}

struct MemoryUnderTest {
    decoder: MemoryPreviewDecoder,
}

impl MemoryUnderTest {
    fn new(image: DecodedImage) -> Self {
        let mut decoder = MemoryPreviewDecoder::new();
        decoder.insert_image(RAF, image);
        decoder.insert_unsupported(GPR);
        decoder.insert_corrupt(TRUNCATED, "unexpected end of file");
        MemoryUnderTest { decoder }
    }
}

impl PreviewDecoderUnderTest for MemoryUnderTest {
    fn decoder(&self) -> &dyn PreviewDecoder {
        &self.decoder
    }

    fn supported_file(&self) -> PathBuf {
        PathBuf::from(RAF)
    }

    fn unsupported_file(&self) -> PathBuf {
        PathBuf::from(GPR)
    }

    fn truncated_file(&self) -> PathBuf {
        PathBuf::from(TRUNCATED)
    }

    fn missing_path(&self) -> PathBuf {
        PathBuf::from("/photos/missing.RAF")
    }
}

/// spec: PREVIEWS-001.1
#[test]
fn a_decoded_image_holds_three_samples_per_pixel() {
    let image = image().unwrap();

    assert_eq!((image.width(), image.height()), (2, 1));
    assert_eq!(image.samples().len(), 6);
}

/// spec: PREVIEWS-001.1
#[test]
fn samples_that_do_not_fit_the_size_are_rejected() {
    let error = DecodedImage::new(2, 1, vec![0; 5]).unwrap_err();

    assert_eq!(
        error.to_string(),
        "5 samples do not fit an image of 2 × 1 pixels with 3 samples each"
    );
}

/// spec: PREVIEWS-001.1
#[test]
fn an_image_without_pixels_is_rejected() {
    assert!(DecodedImage::new(0, 4, Vec::new()).is_err());
    assert!(DecodedImage::new(4, 0, Vec::new()).is_err());
}

#[test]
fn sample_bytes_are_little_endian_in_sample_order() {
    let image = DecodedImage::new(1, 1, vec![0x0102, 0xfffe, 0]).unwrap();

    assert_eq!(image.sample_bytes(), [0x02, 0x01, 0xfe, 0xff, 0, 0]);
}

/// spec: PREVIEWS-001.8
#[test]
fn a_missing_file_names_its_path() {
    let decoder = MemoryPreviewDecoder::new();

    let error = decoder
        .decode_full(Path::new("/photos/missing.RAF"), DecodeMode::Reference)
        .unwrap_err();

    assert!(matches!(error, DecodeError::Io { .. }));
    assert!(
        error
            .to_string()
            .starts_with("cannot read /photos/missing.RAF: ")
    );
}

/// spec: PREVIEWS-001.6
#[test]
fn an_unsupported_format_names_the_file() {
    let decoder = MemoryUnderTest::new(image().unwrap()).decoder;

    let error = decoder
        .decode_full(Path::new(GPR), DecodeMode::Interactive)
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "cannot decode /photos/GOPR0001.GPR: the format is not supported"
    );
}

/// spec: PREVIEWS-001.7
#[test]
fn a_corrupt_file_names_the_file_and_the_decoder_message() {
    let decoder = MemoryUnderTest::new(image().unwrap()).decoder;

    let error = decoder
        .decode_full(Path::new(TRUNCATED), DecodeMode::Interactive)
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "cannot decode /photos/DSCF0022-half.RAF: the file is corrupt: unexpected end of file"
    );
}

/// spec: PREVIEWS-001.2
/// spec: PREVIEWS-001.3
/// spec: PREVIEWS-001.6
/// spec: PREVIEWS-001.7
/// spec: PREVIEWS-001.8
#[test]
fn the_memory_preview_decoder_conforms_to_the_port() {
    let failures = check_preview_decoder(&MemoryUnderTest::new(image().unwrap()));

    assert_eq!(failures, Vec::<String>::new());
}
