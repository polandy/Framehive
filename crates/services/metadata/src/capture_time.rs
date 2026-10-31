// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Finding the EXIF by the container's magic bytes (ADR 0021, decision 2).

use domain::CaptureTime;

use crate::NoCaptureTime;
use crate::source::{ByteSource, Failure, Reader, damaged};
use crate::{jpeg, raf, tiff};

/// Enough bytes to tell the containers apart; the RAF magic is the longest.
const MAGIC_LENGTH: u64 = 16;

/// Reads the capture time from a file's content in memory (CORE-005).
///
/// The container is recognised by its first bytes: JPEG and MPO, TIFF and the
/// TIFF-based RAW formats in either byte order, and RAF. Any other content has no
/// capture time, with [`NoCaptureTime::FormatNotRead`].
///
/// # Errors
///
/// Returns why the content has no capture time.
///
/// ```
/// use metadata::{NoCaptureTime, capture_time_from_bytes};
///
/// let png = b"\x89PNG\r\n\x1a\n";
/// assert_eq!(capture_time_from_bytes(png), Err(NoCaptureTime::FormatNotRead));
/// ```
pub fn capture_time_from_bytes(content: &[u8]) -> Result<CaptureTime, NoCaptureTime> {
    let mut source = content;
    match capture_time(&mut source) {
        Ok(time) => Ok(time),
        Err(Failure::Reason(reason)) => Err(reason),
        // Bytes in memory are read only inside their length, so this does not happen.
        Err(Failure::Io(error)) => Err(NoCaptureTime::damaged(error.to_string())),
    }
}

/// The capture time of the file `source` holds, by its container.
pub(crate) fn capture_time(source: &mut dyn ByteSource) -> Result<CaptureTime, Failure> {
    let mut reader = Reader::new(source);
    let file = reader.file();
    if file.length() == 0 {
        return Err(damaged("the file is empty"));
    }
    let magic_length = usize::try_from(file.length().min(MAGIC_LENGTH)).unwrap_or_default();
    let magic = reader.read(file, 0, magic_length, "file header")?;
    if jpeg::is_jpeg(&magic) {
        jpeg::capture_time(&mut reader, file)
    } else if raf::is_raf(&magic) {
        raf::capture_time(&mut reader, file)
    } else if tiff::is_tiff(&magic) {
        tiff::capture_time(&mut reader, file)
    } else {
        Err(NoCaptureTime::FormatNotRead.into())
    }
}
