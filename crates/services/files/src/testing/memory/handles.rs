// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Open files of a [`MemoryFileSystem`](super::MemoryFileSystem).

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use domain::Timestamp;

use super::Fault;
use super::state::{Entry, FileId, State, lock};
use crate::WriteFile;

/// Reads a snapshot of a file's content taken when it was opened.
pub(super) struct MemoryReader {
    pub(super) state: Arc<Mutex<State>>,
    pub(super) content: Vec<u8>,
    pub(super) position: usize,
    pub(super) readable: usize,
}

impl Read for MemoryReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        lock(&self.state).count_operation()?;
        if self.position >= self.readable && self.position < self.content.len() {
            return Err(io::Error::other("injected read error"));
        }
        let end = self.content.len().min(self.readable);
        let available = &self.content[self.position.min(end)..end];
        let count = available.len().min(buffer.len());
        buffer[..count].copy_from_slice(&available[..count]);
        self.position += count;
        Ok(count)
    }
}

impl Seek for MemoryReader {
    /// Seeks like [`std::fs::File`]: past the end is allowed, before the start is not.
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        lock(&self.state).count_operation()?;
        let target = match position {
            SeekFrom::Start(offset) => usize::try_from(offset).ok(),
            SeekFrom::End(delta) => self.content.len().checked_add_signed(delta_of(delta)?),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta_of(delta)?),
        }
        .ok_or_else(seek_out_of_range)?;
        self.position = target;
        u64::try_from(target).map_err(|_| seek_out_of_range())
    }
}

fn delta_of(delta: i64) -> io::Result<isize> {
    isize::try_from(delta).map_err(|_| seek_out_of_range())
}

fn seek_out_of_range() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "seek to a position before the start or out of range",
    )
}

/// Writes to one file, which stays the same file if it is renamed.
pub(super) struct MemoryWriter {
    pub(super) state: Arc<Mutex<State>>,
    pub(super) id: FileId,
    pub(super) fault: Option<Fault>,
}

impl MemoryWriter {
    fn let_other_program_create_its_file(&mut self, state: &mut State) {
        if !matches!(self.fault, Some(Fault::OtherProgramCreates { .. })) {
            return;
        }
        if let Some(Fault::OtherProgramCreates { path, content }) = self.fault.take() {
            let id = state.new_file(content, Timestamp::UNIX_EPOCH);
            state.entries.insert(path, Entry::File(id));
        }
    }
}

impl Write for MemoryWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let state_handle = Arc::clone(&self.state);
        let mut state = lock(&state_handle);
        state.count_operation()?;
        self.let_other_program_create_its_file(&mut state);
        let file = state.file_mut(self.id);
        let room = match self.fault {
            Some(Fault::WriteFails { after_bytes }) => {
                after_bytes.saturating_sub(file.content.len())
            }
            _ => usize::MAX,
        };
        if room == 0 && !bytes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::StorageFull,
                "no space left on the device",
            ));
        }
        let count = bytes.len().min(room);
        file.content.extend_from_slice(&bytes[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl WriteFile for MemoryWriter {
    fn set_modification_time(&mut self, time: Timestamp) -> io::Result<()> {
        let mut state = lock(&self.state);
        state.count_operation()?;
        state.file_mut(self.id).modified = time;
        Ok(())
    }

    fn sync(&mut self) -> io::Result<()> {
        let mut state = lock(&self.state);
        state.count_operation()?;
        state.make_file_durable(self.id);
        Ok(())
    }
}
