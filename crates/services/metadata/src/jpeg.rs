// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The EXIF of a JPEG (and MPO): the first `APP1` segment that starts with
//! `Exif\0\0`, before the image data (CIPA DC-008 §4.5.4).

use domain::CaptureTime;

use crate::NoCaptureTime;
use crate::source::{Failure, Reader, Window, damaged};
use crate::tiff;

const MARKER_PREFIX: u8 = 0xFF;
const START_OF_IMAGE: [u8; 2] = [0xFF, 0xD8];
const START_OF_SCAN: u8 = 0xDA;
const END_OF_IMAGE: u8 = 0xD9;
const APP1: u8 = 0xE1;
const EXIF_HEADER: &[u8; 6] = b"Exif\0\0";
/// Two bytes of marker, two bytes of segment length.
const SEGMENT_HEADER_LENGTH: u64 = 4;

/// Whether `header` starts like a JPEG.
pub(crate) fn is_jpeg(header: &[u8]) -> bool {
    header.starts_with(&START_OF_IMAGE)
}

/// The capture time of the JPEG that fills `window`.
pub(crate) fn capture_time(
    reader: &mut Reader<'_>,
    window: Window,
) -> Result<CaptureTime, Failure> {
    let start = reader.read(window, 0, 2, "JPEG start")?;
    if !is_jpeg(&start) {
        return Err(damaged("the JPEG start marker is missing"));
    }
    let exif = exif_window(reader, window)?;
    tiff::capture_time(reader, exif)
}

/// The TIFF data of the first EXIF `APP1` segment, walking the segments before the
/// image data.
fn exif_window(reader: &mut Reader<'_>, window: Window) -> Result<Window, Failure> {
    let mut position: u64 = 2;
    loop {
        let marker = reader.read(window, position, 2, "JPEG marker")?;
        if marker[0] != MARKER_PREFIX {
            return Err(damaged(format!("no JPEG marker at offset {position}")));
        }
        match marker[1] {
            // A fill byte before a marker.
            MARKER_PREFIX => {
                position += 1;
                continue;
            }
            // Markers without a segment: TEM and the restart markers.
            0x01 | 0xD0..=0xD7 => {
                position += 2;
                continue;
            }
            START_OF_SCAN | END_OF_IMAGE => return Err(NoCaptureTime::Missing.into()),
            _ => {}
        }
        let length_bytes = reader.read(window, position + 2, 2, "JPEG segment length")?;
        let length = u64::from(u16::from_be_bytes([length_bytes[0], length_bytes[1]]));
        if length < 2 {
            return Err(damaged(format!(
                "JPEG segment at offset {position} has length {length}"
            )));
        }
        let segment = window.part(position + 2, length, "JPEG segment")?;
        if marker[1] == APP1 && starts_with_exif_header(reader, window, position, length)? {
            let header_end = SEGMENT_HEADER_LENGTH + EXIF_HEADER.len() as u64;
            return window.part(position + header_end, length + 2 - header_end, "EXIF data");
        }
        position += 2 + segment.length();
    }
}

fn starts_with_exif_header(
    reader: &mut Reader<'_>,
    window: Window,
    position: u64,
    length: u64,
) -> Result<bool, Failure> {
    let header_length = EXIF_HEADER.len();
    if length < 2 + header_length as u64 {
        return Ok(false);
    }
    let header = reader.read(
        window,
        position + SEGMENT_HEADER_LENGTH,
        header_length,
        "APP1 header",
    )?;
    Ok(header == EXIF_HEADER)
}
