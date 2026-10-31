// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The EXIF of a Fujifilm RAF: its embedded JPEG, whose offset and length the header
//! holds as big-endian numbers at bytes 84 and 88 (ADR 0021).

use domain::CaptureTime;

use crate::jpeg;
use crate::source::{Failure, Reader, Window};

const MAGIC: &[u8; 16] = b"FUJIFILMCCD-RAW ";
const JPEG_POINTER_OFFSET: u64 = 84;

/// Whether `header` starts like a RAF.
pub(crate) fn is_raf(header: &[u8]) -> bool {
    header.starts_with(MAGIC)
}

/// The capture time of the RAF that fills `window`.
pub(crate) fn capture_time(
    reader: &mut Reader<'_>,
    window: Window,
) -> Result<CaptureTime, Failure> {
    let pointer = reader.read(window, JPEG_POINTER_OFFSET, 8, "RAF header")?;
    let offset = u32::from_be_bytes([pointer[0], pointer[1], pointer[2], pointer[3]]);
    let length = u32::from_be_bytes([pointer[4], pointer[5], pointer[6], pointer[7]]);
    let embedded = window.part(offset.into(), length.into(), "embedded JPEG")?;
    jpeg::capture_time(reader, embedded)
}
