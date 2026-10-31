// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Bounded reads of byte ranges from a file (ADR 0021, decisions 3 and 4).
//!
//! Every read is checked against the file size and counts against a budget, so a
//! crafted file can neither point the parser outside itself nor make it read much.

use std::io;

use crate::NoCaptureTime;

/// The most bytes read from one file. The capture time needs a few kilobytes; the
/// limit keeps a damaged file from making the reader walk a large RAW (CORE-005.12).
pub(crate) const MAX_BYTES_READ: u64 = 256 * 1024;

/// A file, or bytes in memory, that can be read at any offset.
pub(crate) trait ByteSource {
    /// The size in bytes.
    fn size(&self) -> u64;

    /// Fills `buffer` from the bytes at `offset`; the caller has checked the range.
    fn read_exact_at(&mut self, offset: u64, buffer: &mut [u8]) -> io::Result<()>;
}

impl ByteSource for &[u8] {
    fn size(&self) -> u64 {
        // A slice is never longer than `u64::MAX` on supported platforms.
        u64::try_from(self.len()).unwrap_or(u64::MAX)
    }

    fn read_exact_at(&mut self, offset: u64, buffer: &mut [u8]) -> io::Result<()> {
        let start = usize::try_from(offset).map_err(|_| io::ErrorKind::UnexpectedEof)?;
        let bytes = start
            .checked_add(buffer.len())
            .and_then(|end| self.get(start..end))
            .ok_or(io::ErrorKind::UnexpectedEof)?;
        buffer.copy_from_slice(bytes);
        Ok(())
    }
}

/// Why reading the capture time stopped: a reason for the file, or an I/O error.
#[derive(Debug)]
pub(crate) enum Failure {
    /// The file has no capture time, for this reason.
    Reason(NoCaptureTime),
    /// The file system failed to read the file.
    Io(io::Error),
}

impl From<NoCaptureTime> for Failure {
    fn from(reason: NoCaptureTime) -> Self {
        Failure::Reason(reason)
    }
}

/// Shorthand for a [`Failure`] for a damaged file.
pub(crate) fn damaged(what: impl Into<String>) -> Failure {
    Failure::Reason(NoCaptureTime::damaged(what))
}

/// A byte range of the file, such as the TIFF data in a JPEG's `APP1` segment.
/// Offsets inside a container are relative to the start of its window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Window {
    start: u64,
    length: u64,
}

impl Window {
    /// The part of this window from `offset` with `length` bytes, named `what` in the
    /// reason if it does not fit.
    pub(crate) fn part(self, offset: u64, length: u64, what: &str) -> Result<Window, Failure> {
        let end = offset.checked_add(length);
        if end.is_none_or(|end| end > self.length) {
            return Err(damaged(format!(
                "{what} at offset {offset} ({length} bytes) is outside the file"
            )));
        }
        Ok(Window {
            start: self.start + offset,
            length,
        })
    }

    /// The number of bytes in the window.
    pub(crate) fn length(self) -> u64 {
        self.length
    }
}

/// Reads byte ranges from a [`ByteSource`] within the file size and the read budget.
pub(crate) struct Reader<'s> {
    source: &'s mut dyn ByteSource,
    bytes_read: u64,
}

impl<'s> Reader<'s> {
    /// A reader that has read nothing yet.
    pub(crate) fn new(source: &'s mut dyn ByteSource) -> Self {
        Reader {
            source,
            bytes_read: 0,
        }
    }

    /// The whole file.
    pub(crate) fn file(&self) -> Window {
        Window {
            start: 0,
            length: self.source.size(),
        }
    }

    /// Reads `length` bytes at `offset` of `window`; `what` names them in the reason
    /// if they are outside the window.
    pub(crate) fn read(
        &mut self,
        window: Window,
        offset: u64,
        length: usize,
        what: &str,
    ) -> Result<Vec<u8>, Failure> {
        let wanted = u64::try_from(length).unwrap_or(u64::MAX);
        let range = window.part(offset, wanted, what)?;
        let bytes_read = self.bytes_read.saturating_add(wanted);
        if bytes_read > MAX_BYTES_READ {
            return Err(damaged(format!(
                "more than {MAX_BYTES_READ} bytes of metadata"
            )));
        }
        self.bytes_read = bytes_read;
        let mut buffer = vec![0; length];
        self.source
            .read_exact_at(range.start, &mut buffer)
            .map_err(Failure::Io)?;
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason(result: Result<Vec<u8>, Failure>) -> String {
        match result {
            Err(Failure::Reason(reason)) => reason.to_string(),
            other => format!("{other:?}"),
        }
    }

    #[test]
    fn reads_inside_a_window() {
        let mut bytes: &[u8] = b"0123456789";
        let mut reader = Reader::new(&mut bytes);
        let window = reader.file().part(2, 6, "part").unwrap();

        assert_eq!(reader.read(window, 1, 3, "value").unwrap(), b"345");
        assert_eq!(
            reason(reader.read(window, 4, 3, "value")),
            "damaged: value at offset 4 (3 bytes) is outside the file"
        );
        assert_eq!(
            reason(reader.read(window, u64::MAX, 1, "value")),
            "damaged: value at offset 18446744073709551615 (1 bytes) is outside the file"
        );
    }

    #[test]
    fn stops_at_the_read_budget() {
        let content = vec![0; 200 * 1024];
        let mut bytes: &[u8] = &content;
        let mut reader = Reader::new(&mut bytes);
        let file = reader.file();

        assert!(reader.read(file, 0, 200 * 1024, "first").is_ok());
        assert_eq!(
            reason(reader.read(file, 0, 100 * 1024, "second")),
            "damaged: more than 262144 bytes of metadata"
        );
    }
}
