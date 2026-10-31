// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading the capture time of a file through the `FileSystem` port (CORE-005).

use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use domain::CaptureTime;
use files::{FileSystem, ReadSeek};

use crate::NoCaptureTime;
use crate::capture_time::capture_time;
use crate::source::{ByteSource, Failure};

/// The extensions of the formats in the CORE-005 table, in lower case. A file with
/// any other extension is not opened.
const READ_EXTENSIONS: [&str; 21] = [
    "jpg", "jpeg", "mpo", // JPEG
    "dng", "cr2", "nef", "nrw", "arw", "sr2", "pef", "orf", "rw2", "3fr", "dcr", "fff", "iiq",
    "mef", "mos", "srw", "tif", "tiff", // TIFF-based
];
const RAF_EXTENSION: &str = "raf";

/// Why [`read_capture_time`] gives no capture time.
#[derive(Debug, thiserror::Error)]
pub enum CaptureTimeError {
    /// The file was read and has no capture time, for this reason.
    #[error(transparent)]
    NoCaptureTime(#[from] NoCaptureTime),
    /// The file system failed to open or read the file.
    #[error("cannot read {}", path.display())]
    Io {
        /// The file that could not be read.
        path: PathBuf,
        /// The error of the file system.
        #[source]
        error: io::Error,
    },
}

/// Reads the capture time of the file at `path` (CORE-005).
///
/// Only files whose extension, in any case, names a format in the CORE-005 table are
/// opened; any other file gives [`NoCaptureTime::FormatNotRead`] without being read.
/// The container is then recognised by its first bytes, and only the few kilobytes
/// that lead to the EXIF are read, never the whole file.
///
/// # Errors
///
/// Returns why the file has no capture time, or the I/O error that stopped reading it.
///
/// ```
/// use std::path::Path;
///
/// use domain::Timestamp;
/// use files::testing::MemoryFileSystem;
/// use metadata::testing::{ByteOrder, DATE_TIME_ORIGINAL, Tiff};
/// use metadata::read_capture_time;
///
/// let raw = Tiff::new(ByteOrder::LittleEndian)
///     .with_exif_text(DATE_TIME_ORIGINAL, "2026:10:17 14:05:09")
///     .to_bytes();
/// let file_system = MemoryFileSystem::new();
/// file_system.add_file(Path::new("/card/DSC_0001.NEF"), &raw, Timestamp::UNIX_EPOCH);
///
/// let time = read_capture_time(&file_system, Path::new("/card/DSC_0001.NEF"))?;
/// assert_eq!(time.date_time().minute(), 5);
/// # Ok::<(), metadata::CaptureTimeError>(())
/// ```
pub fn read_capture_time(
    file_system: &dyn FileSystem,
    path: &Path,
) -> Result<CaptureTime, CaptureTimeError> {
    if !is_read(path) {
        return Err(NoCaptureTime::FormatNotRead.into());
    }
    let io_error = |error| CaptureTimeError::Io {
        path: path.to_owned(),
        error,
    };
    let file = file_system.open_read(path).map_err(io_error)?;
    let mut source = FileSource::new(file).map_err(io_error)?;
    capture_time(&mut source).map_err(|failure| match failure {
        Failure::Reason(reason) => reason.into(),
        Failure::Io(error) => io_error(error),
    })
}

/// Whether the extension of `path` names a format CORE-005 reads.
fn is_read(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| {
            extension == RAF_EXTENSION || READ_EXTENSIONS.contains(&extension.as_str())
        })
}

/// An open file read at any offset.
struct FileSource {
    file: Box<dyn ReadSeek>,
    size: u64,
}

impl FileSource {
    fn new(mut file: Box<dyn ReadSeek>) -> io::Result<Self> {
        let size = file.seek(SeekFrom::End(0))?;
        Ok(FileSource { file, size })
    }
}

impl ByteSource for FileSource {
    fn size(&self) -> u64 {
        self.size
    }

    fn read_exact_at(&mut self, offset: u64, buffer: &mut [u8]) -> io::Result<()> {
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_are_compared_in_any_case() {
        for read in [
            "a.jpg", "a.JPG", "a.Jpeg", "a.RAF", "a.raf", "a.NEF", "a.3FR",
        ] {
            assert!(is_read(Path::new(read)), "{read}");
        }
        for not_read in ["a.MOV", "a.CR3", "a.HIF", "a.png", "jpg", "a", "a.jpg.xmp"] {
            assert!(!is_read(Path::new(not_read)), "{not_read}");
        }
    }
}
