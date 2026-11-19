// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A [`ReadOnlyFileSystem`] that gives every call to a source a stall limit
//! (ADR 0049 §1, §2).

use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use domain::Timestamp;

use crate::{
    AnswerSignal, EntryKind, FolderEntry, ReadOnlyFileSystem, ReadSeek, Timer, WaitOutcome,
};

/// Why a call to a source behind a [`StallGuard`] failed without an answer of the
/// source. [`source_stall`] finds it in an [`io::Error`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SourceStall {
    /// The call did not answer within the stall limit, so the source is stopped.
    #[error("the source did not answer for {limit_seconds} seconds")]
    DidNotAnswer {
        /// The stall limit in whole seconds.
        limit_seconds: u64,
    },
    /// An earlier call did not answer, so this call was not made.
    #[error("the source stopped answering")]
    StoppedAnswering,
}

/// Returns the [`SourceStall`] that `error` carries, or `None` for any other error.
#[must_use]
pub fn source_stall(error: &io::Error) -> Option<&SourceStall> {
    error.get_ref()?.downcast_ref()
}

/// A [`ReadOnlyFileSystem`] that makes every call of its source on a thread of its
/// own and stops waiting for a call that does not answer within the stall limit
/// (ADR 0049).
///
/// Such a call returns an [`io::ErrorKind::TimedOut`] error with
/// [`SourceStall::DidNotAnswer`], and the source is *stopped*: every later call returns
/// [`SourceStall::StoppedAnswering`] at once, without a call to the source. The call
/// that did not answer stays blocked on the source thread, which nothing joins.
/// Dropping the guard and its open files ends the source thread once it is not
/// blocked, which drops the source.
pub struct StallGuard {
    calls: Arc<Calls>,
}

/// What a [`StallGuard`] and its open files share.
struct Calls {
    requests: Sender<Request>,
    timer: Arc<dyn Timer>,
    limit: Duration,
    stopped: AtomicBool,
}

type Request = Box<dyn FnOnce(&mut Source) + Send>;

/// The source and its open files, owned by the source thread.
struct Source {
    file_system: Box<dyn ReadOnlyFileSystem + Send>,
    open_files: HashMap<u64, Box<dyn ReadSeek>>,
    next_file: u64,
}

/// Where the source thread puts the answer to one call.
struct AnswerSlot<T> {
    value: Mutex<Option<io::Result<T>>>,
    signal: AnswerSignal,
}

/// The source thread's end of an [`AnswerSlot`]. Dropping it signals, so a request
/// that is dropped unanswered never leaves its caller waiting.
struct Reply<T>(Arc<AnswerSlot<T>>);

impl StallGuard {
    /// Starts the source thread for `source`, whose every call must answer within
    /// `limit` as `timer` measures it.
    ///
    /// # Errors
    ///
    /// Returns an error if the thread cannot be started.
    pub fn new(
        source: Box<dyn ReadOnlyFileSystem + Send>,
        timer: Arc<dyn Timer>,
        limit: Duration,
    ) -> io::Result<Self> {
        let (requests, received) = mpsc::channel::<Request>();
        thread::Builder::new()
            .name("source".to_owned())
            .spawn(move || {
                let mut source = Source {
                    file_system: source,
                    open_files: HashMap::new(),
                    next_file: 0,
                };
                for request in received {
                    request(&mut source);
                }
            })?;
        let calls = Calls {
            requests,
            timer,
            limit,
            stopped: AtomicBool::new(false),
        };
        Ok(StallGuard {
            calls: Arc::new(calls),
        })
    }

    /// Whether a call did not answer within the limit, so no further call is made.
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        self.calls.is_stopped()
    }
}

impl ReadOnlyFileSystem for StallGuard {
    fn entry(&self, path: &Path) -> io::Result<Option<EntryKind>> {
        let path = path.to_path_buf();
        self.calls
            .call(move |source| source.file_system.entry(&path))
    }

    fn list_folder(&self, path: &Path) -> io::Result<Vec<FolderEntry>> {
        let path = path.to_path_buf();
        self.calls
            .call(move |source| source.file_system.list_folder(&path))
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        let path = path.to_path_buf();
        self.calls
            .call(move |source| source.file_system.file_size(&path))
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>> {
        let path = path.to_path_buf();
        let file = self.calls.call(move |source| source.open(&path))?;
        Ok(Box::new(GuardedFile {
            calls: Arc::clone(&self.calls),
            file,
        }))
    }

    fn modification_time(&self, path: &Path) -> io::Result<Timestamp> {
        let path = path.to_path_buf();
        self.calls
            .call(move |source| source.file_system.modification_time(&path))
    }
}

impl Calls {
    fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    /// Sends `call` to the source thread and waits for its answer until the limit.
    fn call<T: Send + 'static>(
        &self,
        call: impl FnOnce(&mut Source) -> io::Result<T> + Send + 'static,
    ) -> io::Result<T> {
        if self.is_stopped() {
            return Err(io::Error::other(SourceStall::StoppedAnswering));
        }
        let slot = Arc::new(AnswerSlot {
            value: Mutex::new(None),
            signal: AnswerSignal::new(),
        });
        let reply = Reply(Arc::clone(&slot));
        // Fixed before the request is sent, so the limit counts from the call.
        let deadline = self.timer.deadline(self.limit);
        let request: Request = Box::new(move |source| reply.give(call(source)));
        if self.requests.send(request).is_err() {
            return Err(source_thread_ended());
        }
        match self.timer.wait(&slot.signal, deadline) {
            WaitOutcome::Answered => slot.take(),
            WaitOutcome::DeadlinePassed => {
                self.stopped.store(true, Ordering::SeqCst);
                let stall = SourceStall::DidNotAnswer {
                    limit_seconds: self.limit.as_secs(),
                };
                Err(io::Error::new(io::ErrorKind::TimedOut, stall))
            }
        }
    }
}

impl Source {
    fn open(&mut self, path: &Path) -> io::Result<u64> {
        let opened = self.file_system.open_read(path)?;
        let file = self.next_file;
        self.next_file = self.next_file.wrapping_add(1);
        self.open_files.insert(file, opened);
        Ok(file)
    }

    fn read(&mut self, file: u64, wanted: usize) -> io::Result<Vec<u8>> {
        let mut bytes = vec![0; wanted];
        let count = self.open_file(file)?.read(&mut bytes)?;
        bytes.truncate(count);
        Ok(bytes)
    }

    fn seek(&mut self, file: u64, position: SeekFrom) -> io::Result<u64> {
        self.open_file(file)?.seek(position)
    }

    fn close(&mut self, file: u64) {
        self.open_files.remove(&file);
    }

    fn open_file(&mut self, file: u64) -> io::Result<&mut Box<dyn ReadSeek>> {
        self.open_files
            .get_mut(&file)
            .ok_or_else(|| io::Error::other("the file is not open on the source"))
    }
}

impl<T> AnswerSlot<T> {
    fn take(&self) -> io::Result<T> {
        self.lock()
            .take()
            .unwrap_or_else(|| Err(source_thread_ended()))
    }

    fn lock(&self) -> MutexGuard<'_, Option<io::Result<T>>> {
        // A panic while the value was locked cannot leave it half written.
        self.value.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<T> Reply<T> {
    fn give(self, answer: io::Result<T>) {
        *self.0.lock() = Some(answer);
    }
}

impl<T> Drop for Reply<T> {
    fn drop(&mut self) {
        self.0.signal.signal();
    }
}

fn source_thread_ended() -> io::Error {
    io::Error::other("the source thread ended")
}

/// A file opened through a [`StallGuard`]; the file itself stays on the source thread.
struct GuardedFile {
    calls: Arc<Calls>,
    file: u64,
}

impl Read for GuardedFile {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let (file, wanted) = (self.file, buffer.len());
        let bytes = self.calls.call(move |source| source.read(file, wanted))?;
        let count = bytes.len().min(buffer.len());
        buffer[..count].copy_from_slice(&bytes[..count]);
        Ok(count)
    }
}

impl Seek for GuardedFile {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let file = self.file;
        self.calls.call(move |source| source.seek(file, position))
    }
}

impl Drop for GuardedFile {
    fn drop(&mut self) {
        if self.calls.is_stopped() {
            return;
        }
        let file = self.file;
        let close: Request = Box::new(move |source| source.close(file));
        // A source thread that ended has closed every file already.
        let _ = self.calls.requests.send(close);
    }
}
