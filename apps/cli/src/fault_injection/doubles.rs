// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The readers and the writer the fault-injection point hands out (ADR 0048).

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use domain::Timestamp;
use files::{ReadSeek, WriteFile};

use super::{State, device_gone, lock};

/// Writes half of its first write, then ends the process as a kill does.
pub(super) struct AbortingWriter {
    pub(super) inner: Box<dyn WriteFile>,
}

impl Write for AbortingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // The process ends here, so a failed write has no one to report to.
        let _ = self.inner.write_all(&bytes[..bytes.len() / 2]);
        std::process::abort()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl WriteFile for AbortingWriter {
    fn set_modification_time(&mut self, time: Timestamp) -> io::Result<()> {
        self.inner.set_modification_time(time)
    }

    fn sync(&mut self) -> io::Result<()> {
        self.inner.sync()
    }
}

/// Flips the lowest bit of the first byte it returns.
pub(super) struct BitFlippingReader {
    pub(super) inner: Box<dyn ReadSeek>,
    pub(super) flipped: bool,
}

impl Read for BitFlippingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buffer)?;
        if let Some(first) = buffer[..count].first_mut().filter(|_| !self.flipped) {
            *first ^= 1;
            self.flipped = true;
        }
        Ok(count)
    }
}

impl Seek for BitFlippingReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}

/// Blocks its first read while a `.part` file is written and never returns from it, so
/// reading the metadata or the checksum before the copy is not affected.
pub(super) struct HangingReader {
    pub(super) inner: Box<dyn ReadSeek>,
    pub(super) state: Arc<Mutex<State>>,
}

impl Read for HangingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut state = lock(&self.state);
        if state.copying && !state.fired {
            state.fired = true;
            drop(state);
            hang_forever();
        }
        drop(state);
        self.inner.read(buffer)
    }
}

impl Seek for HangingReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}

/// Blocks the calling thread until the process ends, as a device that stopped answering.
fn hang_forever() -> ! {
    loop {
        // A spurious wake-up must not end the hang.
        std::thread::park();
    }
}

/// Unplugs `folder` at its first read while a `.part` file is written, so reading the
/// metadata or the checksum before the copy is not affected.
pub(super) struct UnpluggingReader {
    pub(super) inner: Box<dyn ReadSeek>,
    pub(super) folder: PathBuf,
    pub(super) state: Arc<Mutex<State>>,
}

impl Read for UnpluggingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut state = lock(&self.state);
        if state.unplugged.is_some() {
            return Err(device_gone());
        }
        if state.copying && !state.fired {
            state.fired = true;
            state.unplugged = Some(self.folder.clone());
            return Err(device_gone());
        }
        drop(state);
        self.inner.read(buffer)
    }
}

impl Seek for UnpluggingReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}
