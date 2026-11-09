// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The camera model of a file, found by the container's magic bytes (IMPORT-021).

use crate::source::{ByteSource, Failure, Reader};
use crate::{jpeg, raf, tiff};

/// Enough bytes to tell the containers apart; the RAF magic is the longest.
const MAGIC_LENGTH: u64 = 16;

/// Reads the camera model from a file's content in memory (IMPORT-021).
///
/// Content in another format, without a `Model` or with a damaged EXIF has none.
///
/// ```
/// use metadata::camera_model_from_bytes;
///
/// assert_eq!(camera_model_from_bytes(b"\x89PNG\r\n\x1a\n"), None);
/// ```
#[must_use]
pub fn camera_model_from_bytes(content: &[u8]) -> Option<String> {
    let mut source = content;
    camera_model(&mut source).ok().flatten()
}

/// The model of the file `source` holds, by its container.
pub(crate) fn camera_model(source: &mut dyn ByteSource) -> Result<Option<String>, Failure> {
    let mut reader = Reader::new(source);
    let file = reader.file();
    if file.length() == 0 {
        return Ok(None);
    }
    let magic_length = usize::try_from(file.length().min(MAGIC_LENGTH)).unwrap_or_default();
    let magic = reader.read(file, 0, magic_length, "file header")?;
    let model = if jpeg::is_jpeg(&magic) {
        jpeg::camera_model(&mut reader, file)
    } else if raf::is_raf(&magic) {
        raf::camera_model(&mut reader, file)
    } else if tiff::is_tiff(&magic) {
        tiff::camera_model(&mut reader, file)
    } else {
        Ok(None)
    };
    match model {
        Ok(model) => Ok(model.and_then(|bytes| text(&bytes))),
        Err(Failure::Reason(_)) => Ok(None),
        Err(failure) => Err(failure),
    }
}

/// The text up to the first NUL; `None` when that is empty.
fn text(bytes: &[u8]) -> Option<String> {
    let bytes = bytes.split(|byte| *byte == 0).next().unwrap_or_default();
    (!bytes.is_empty()).then(|| String::from_utf8_lossy(bytes).into_owned())
}
