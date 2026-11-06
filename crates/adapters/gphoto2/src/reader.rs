// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! [`CameraReader`]: reads a camera file in chunks from any offset (ADR 0034 §3).

use std::ffi::CString;
use std::io::{self, ErrorKind, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::file_system::{io_error, lock};
use crate::session::Session;

/// How many bytes one call to the camera reads at most. A PTP transfer costs a round
/// trip, so small reads are gathered into chunks; a chunk is small enough that a seek
/// to the metadata of a RAW file does not transfer the file.
const CHUNK_BYTES: usize = 1024 * 1024;

/// An open camera file. It keeps the camera open until it is dropped.
pub(crate) struct CameraReader {
    session: Arc<Mutex<Session>>,
    folder: CString,
    name: CString,
    path: PathBuf,
    size: u64,
    position: u64,
    /// The bytes of the file from `chunk_start` on, as last read from the camera.
    chunk: Vec<u8>,
    chunk_start: u64,
}

impl CameraReader {
    pub(crate) fn new(
        session: Arc<Mutex<Session>>,
        folder: CString,
        name: CString,
        size: u64,
        path: &Path,
    ) -> Self {
        Self {
            session,
            folder,
            name,
            path: path.to_path_buf(),
            size,
            position: 0,
            chunk: Vec::new(),
            chunk_start: 0,
        }
    }

    /// The bytes of the chunk from the current position on, or `None` if the chunk
    /// does not hold the position.
    fn chunk_at_position(&self) -> Option<&[u8]> {
        let offset = usize::try_from(self.position.checked_sub(self.chunk_start)?).ok()?;
        self.chunk.get(offset..).filter(|bytes| !bytes.is_empty())
    }

    /// Reads the chunk that starts at the current position from the camera.
    fn read_chunk(&mut self) -> io::Result<()> {
        let remaining = self.size - self.position;
        let length = usize::try_from(remaining).map_or(CHUNK_BYTES, |bytes| bytes.min(CHUNK_BYTES));
        self.chunk.resize(length, 0);
        self.chunk_start = self.position;
        let read =
            lock(&self.session)?.read(&self.folder, &self.name, self.position, &mut self.chunk);
        match read {
            Ok(count) => {
                self.chunk.truncate(count);
                Ok(())
            }
            Err(error) => {
                self.chunk.clear();
                Err(io_error("read", &self.path, error))
            }
        }
    }
}

impl Read for CameraReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() || self.position >= self.size {
            return Ok(0);
        }
        if self.chunk_at_position().is_none() {
            self.read_chunk()?;
        }
        // A file that ends before its reported size is an error, never a short file:
        // the copy and its verification would both see the same truncated bytes.
        let Some(bytes) = self.chunk_at_position() else {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                format!(
                    "{} on the camera ended before its reported size of {} bytes",
                    self.path.display(),
                    self.size
                ),
            ));
        };
        let count = bytes.len().min(buffer.len());
        buffer[..count].copy_from_slice(&bytes[..count]);
        self.position += count as u64;
        Ok(count)
    }
}

impl Seek for CameraReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let target = match position {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::End(delta) => self.size.checked_add_signed(delta),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta),
        };
        let target = target.ok_or_else(|| {
            io::Error::new(
                ErrorKind::InvalidInput,
                format!("cannot seek before the start of {}", self.path.display()),
            )
        })?;
        self.position = target;
        Ok(target)
    }
}
