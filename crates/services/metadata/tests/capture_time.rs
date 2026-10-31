// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The capture time from the EXIF of JPEG, TIFF-based and RAF content (CORE-005).

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

use domain::{CaptureTime, LocalDateTime};
use metadata::testing::{
    ByteOrder, DATE_TIME, DATE_TIME_ORIGINAL, OFFSET_TIME_ORIGINAL, SUB_SEC_TIME_ORIGINAL, Tiff,
    jpeg, raf,
};
use metadata::{NoCaptureTime, capture_time_from_bytes};

const TAKEN: &str = "2026:10:17 14:05:09";

fn taken() -> LocalDateTime {
    LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap()
}

fn exif(date_time_original: &str) -> Tiff {
    Tiff::new(ByteOrder::LittleEndian).with_exif_text(DATE_TIME_ORIGINAL, date_time_original)
}

fn reason(content: &[u8]) -> String {
    capture_time_from_bytes(content).unwrap_err().to_string()
}

/// spec: CORE-005.1
#[test]
fn jpeg_gives_its_date_time_original() {
    let content = jpeg(&exif(TAKEN).to_bytes());

    assert_eq!(
        capture_time_from_bytes(&content),
        Ok(CaptureTime::new(taken()))
    );
}

/// spec: CORE-005.2
#[test]
fn jpeg_gives_sub_seconds_and_offset() {
    let tiff = exif(TAKEN)
        .with_exif_text(SUB_SEC_TIME_ORIGINAL, "42")
        .with_exif_text(OFFSET_TIME_ORIGINAL, "+02:00");

    let time = capture_time_from_bytes(&jpeg(&tiff.to_bytes())).unwrap();

    assert_eq!(time.date_time(), taken());
    assert_eq!(time.subsec_nanoseconds(), Some(420_000_000));
    assert_eq!(
        time.offset().map(|offset| offset.to_string()).as_deref(),
        Some("+02:00")
    );
}

/// spec: CORE-005.3
#[test]
fn tiff_based_raw_is_read_in_both_byte_orders() {
    for order in [ByteOrder::LittleEndian, ByteOrder::BigEndian] {
        let content = Tiff::new(order)
            .with_exif_text(DATE_TIME_ORIGINAL, TAKEN)
            .to_bytes();

        assert_eq!(
            capture_time_from_bytes(&content),
            Ok(CaptureTime::new(taken())),
            "{order:?}"
        );
    }
}

/// spec: CORE-005.3
#[test]
fn orf_and_rw2_variants_of_the_magic_are_read() {
    for (order, magic) in [
        (ByteOrder::LittleEndian, 0x4F52),
        (ByteOrder::LittleEndian, 0x5352),
        (ByteOrder::BigEndian, 0x4F52),
        (ByteOrder::LittleEndian, 0x0055),
    ] {
        let content = Tiff::new(order)
            .with_magic(magic)
            .with_exif_text(DATE_TIME_ORIGINAL, TAKEN)
            .to_bytes();

        assert_eq!(
            capture_time_from_bytes(&content),
            Ok(CaptureTime::new(taken())),
            "{order:?} {magic:#06x}"
        );
    }
}

/// spec: CORE-005.4
#[test]
fn raf_is_read_through_its_embedded_jpeg() {
    let content = raf(&jpeg(&exif(TAKEN).to_bytes()));

    assert_eq!(
        capture_time_from_bytes(&content),
        Ok(CaptureTime::new(taken()))
    );
}

/// spec: CORE-005.5
#[test]
fn date_time_is_not_a_capture_time() {
    let tiff = Tiff::new(ByteOrder::LittleEndian).with_ifd0_text(DATE_TIME, "2026:10:18 09:00:00");

    assert_eq!(reason(&jpeg(&tiff.to_bytes())), "no capture time");
}

/// spec: CORE-005.6
#[test]
fn unknown_values_are_no_capture_time() {
    for unknown in ["0000:00:00 00:00:00", "                   "] {
        assert_eq!(reason(&jpeg(&exif(unknown).to_bytes())), "no capture time");
    }
}

/// spec: CORE-005.7
#[test]
fn invalid_values_are_named_in_the_reason() {
    let content = jpeg(&exif("2026:13:40 25:00:00").to_bytes());

    assert_eq!(
        reason(&content),
        "invalid capture time: 2026:13:40 25:00:00"
    );
}

/// spec: CORE-005.8
#[test]
fn invalid_sub_seconds_and_offset_are_ignored() {
    let tiff = exif(TAKEN)
        .with_exif_text(SUB_SEC_TIME_ORIGINAL, "4x")
        .with_exif_text(OFFSET_TIME_ORIGINAL, "+25:00");

    assert_eq!(
        capture_time_from_bytes(&jpeg(&tiff.to_bytes())),
        Ok(CaptureTime::new(taken()))
    );
}

/// spec: CORE-005.9
#[test]
fn jpeg_cut_off_inside_its_exif_is_damaged() {
    let mut content = jpeg(&exif(TAKEN).to_bytes());
    let exif_start = content
        .windows(6)
        .position(|bytes| bytes == b"Exif\0\0")
        .unwrap();
    content.truncate(exif_start + 20);

    assert!(
        reason(&content).starts_with("damaged"),
        "{}",
        reason(&content)
    );
}

/// spec: CORE-005.9
#[test]
fn ifd_outside_the_file_is_damaged() {
    let mut content = exif(TAKEN).to_bytes();
    content[4..8].copy_from_slice(&1_000_000_u32.to_le_bytes());

    assert_eq!(
        reason(&content),
        "damaged: IFD at offset 1000000 (2 bytes) is outside the file"
    );
}

/// spec: CORE-005.9
#[test]
fn ifd_pointing_to_itself_is_damaged() {
    let content = exif(TAKEN).with_exif_ifd_pointer(8).to_bytes();

    assert_eq!(reason(&content), "damaged: IFD at offset 8 is read twice");
}

#[test]
fn other_content_is_a_format_not_read() {
    assert_eq!(
        capture_time_from_bytes(b"\0\0\0\x18ftypheic"),
        Err(NoCaptureTime::FormatNotRead)
    );
    assert_eq!(reason(b""), "damaged: the file is empty");
}

#[test]
fn jpeg_without_exif_has_no_capture_time() {
    let mut content = jpeg(&exif(TAKEN).to_bytes());
    let exif_start = content
        .windows(6)
        .position(|bytes| bytes == b"Exif\0\0")
        .unwrap();
    content[exif_start] = b'X';

    assert_eq!(reason(&content), "no capture time");
}

#[test]
fn raf_pointing_outside_the_file_is_damaged() {
    let mut content = raf(&jpeg(&exif(TAKEN).to_bytes()));
    content[88..92].copy_from_slice(&u32::MAX.to_be_bytes());

    assert!(reason(&content).starts_with("damaged: embedded JPEG"));
}
