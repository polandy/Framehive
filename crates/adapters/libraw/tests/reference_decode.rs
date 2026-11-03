// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The version test of ADR 0012 §3: the pinned LibRaw decodes every supported RAW in
//! `testdata.lock` in reference mode to the stored size and pixel hash (PREVIEWS-001).
//! One test per file, so that the test runner decodes them in parallel.

#![allow(clippy::unwrap_used, reason = "a test fails on any decode error")]

mod testdata;

use adapter_libraw::LibRawDecoder;
use previews::{DecodeMode, PreviewDecoder};

/// The decoder version the reference values below were stored for.
const PINNED_VERSION: &str = "LibRaw 0.22.1";

/// Whether the pixel hashes below were measured on this platform; elsewhere only the
/// size is compared until hashes for it are stored (PREVIEWS-001).
const HASHES_MEASURED: bool = cfg!(all(target_os = "linux", target_arch = "x86_64"));

/// Decodes `relative` in reference mode and compares it with the stored values.
fn assert_reference(relative: &str, width: u32, height: u32, hash: &str) {
    let decoder = LibRawDecoder::new();
    assert_eq!(
        decoder.version(),
        PINNED_VERSION,
        "the reference values are for {PINNED_VERSION}"
    );

    let image = decoder
        .decode_full(&testdata::path(relative), DecodeMode::Reference)
        .unwrap();

    assert_eq!(
        (image.width(), image.height()),
        (width, height),
        "{relative}"
    );
    if HASHES_MEASURED {
        let actual = blake3::hash(&image.sample_bytes()).to_hex();
        assert_eq!(actual.as_str(), hash, "pixel hash of {relative}");
    }
}

/// spec: PREVIEWS-001.5
#[test]
fn the_decoder_is_the_pinned_libraw_version() {
    assert_eq!(LibRawDecoder::new().version(), PINNED_VERSION);
}

/// spec: PREVIEWS-001.4
#[test]
fn x_t5_lossless_compressed_raf_matches_its_reference() {
    assert_reference(
        "raw.pixls.us/Fujifilm/X-T5/DSCF0021.RAF",
        7752,
        5178,
        "937554ba45b68b4cdd0469173a1d1c3addb31877496ed27c61a2429498355ce5",
    );
}

/// spec: PREVIEWS-001.4
#[test]
fn x_t5_lossy_compressed_raf_matches_its_reference() {
    assert_reference(
        "raw.pixls.us/Fujifilm/X-T5/DSCF0022.RAF",
        7752,
        5178,
        "33570674a0c7e3cad53eaffe4b653ad5dec78575a0e2b27b066d9ebbe202035e",
    );
}

/// spec: PREVIEWS-001.4
#[test]
fn x_t5_uncompressed_raf_matches_its_reference() {
    assert_reference(
        "raw.pixls.us/Fujifilm/X-T5/DSCF0023.RAF",
        7752,
        5178,
        "61def1a93cfbd886d8c159f0fb19f6ce76cb7b09ce8e4db764cc98fc8ba11ca7",
    );
}

/// spec: PREVIEWS-001.4
#[test]
fn x_t5_sports_finder_crop_raf_matches_its_reference() {
    assert_reference(
        "raw.pixls.us/Fujifilm/X-T5/DSCF4964_70-300.RAF",
        6000,
        3997,
        "9ccc0ffd136cc39c25d289af1c4d335123cb10223fda650c529e4c5405b7b6de",
    );
}

/// spec: PREVIEWS-001.4
#[test]
fn eos_7d_raw_cr2_matches_its_reference() {
    assert_reference(
        "raw.pixls.us/Canon/EOS-7D/RAW_CANON_EOS_7D-raw.CR2",
        5202,
        3464,
        "978323a9ada209c8f61cdfeea926be9c91812a9332219958ead3f8549471907b",
    );
}

/// spec: PREVIEWS-001.4
#[test]
fn eos_7d_m_raw_cr2_matches_its_reference() {
    assert_reference(
        "raw.pixls.us/Canon/EOS-7D/RAW_CANON_EOS_7D-mraw.CR2",
        3888,
        2592,
        "854da8df07cd25dbb3fe99dfbff85848849bb5dde23a7d4757f21eee7ecf8112",
    );
}

/// spec: PREVIEWS-001.4
#[test]
fn eos_7d_s_raw_cr2_matches_its_reference() {
    assert_reference(
        "raw.pixls.us/Canon/EOS-7D/RAW_CANON_EOS_7D-sraw.CR2",
        2592,
        1728,
        "666d02ed5d6cba029371e6161d3660a438fec033578d2fe64bbdc90fa96292ad",
    );
}

/// spec: PREVIEWS-001.4
#[test]
fn lossless_jpeg_dng_matches_its_reference() {
    assert_reference(
        "raw.pixls.us/Adobe-DNG-Converter/Canon-EOS-5D-Mark-III/5G4A9394-compressed-lossless.DNG",
        5796,
        3870,
        "6ae5ac519ff0ee75d967b323e1f0878a74fbd35ca19850610d562f1bc72fe1b7",
    );
}

/// spec: PREVIEWS-001.4
#[test]
fn lossy_jpeg_dng_matches_its_reference() {
    assert_reference(
        "raw.pixls.us/Adobe-DNG-Converter/Canon-EOS-5D-Mark-III/5G4A9394-compressed-lossy.DNG",
        5760,
        3840,
        "0e4652d9be47104fe1952b4d0c376ec9b98a18323f7043aacc5db4b323957a7f",
    );
}

/// spec: PREVIEWS-001.4
#[test]
fn floating_point_deflate_dng_matches_its_reference() {
    assert_reference(
        "raw.pixls.us/Canon/EOS-5D-Mark-III/hdrmerge-bayer-fp16-w-pred-deflate.dng",
        5796,
        3870,
        "4fc38d82fdd6e7e492447506bad7432042e7242dbbdf9b8afcbca59171daab86",
    );
}
