// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Fakes that the tests of `framehive import` and of the guided import share.

use std::io;
use std::path::Path;
use std::thread::{self, ThreadId};

use files::testing::MemoryFreeSpace;
use files::{FreeSpace, SpaceOnFileSystem};

/// An unlimited free space that panics when asked from any thread but the one that made
/// it. The command checks the free space on its own thread and the import job checks it
/// again on the queue's, so the job panics: a fault in the import itself (IMPORT-015),
/// which the tests cannot cause otherwise.
pub(crate) struct PanickingInTheJob {
    answers: MemoryFreeSpace,
    thread: ThreadId,
}

impl PanickingInTheJob {
    pub(crate) fn new() -> Self {
        PanickingInTheJob {
            answers: MemoryFreeSpace::unlimited(),
            thread: thread::current().id(),
        }
    }
}

impl FreeSpace for PanickingInTheJob {
    fn free_space(&self, folder: &Path) -> io::Result<SpaceOnFileSystem> {
        assert_eq!(
            thread::current().id(),
            self.thread,
            "a fault in the import job, on purpose"
        );
        self.answers.free_space(folder)
    }
}

/// An unlimited free space that cannot be read from any thread but the one that made
/// it: the command's own check reads it, the import job's check on the queue's thread
/// does not (CLI-011.9, CLI-011.10).
pub(crate) struct UnreadableInTheJob {
    answers: MemoryFreeSpace,
    thread: ThreadId,
}

impl UnreadableInTheJob {
    /// The reason the job's check gets.
    pub(crate) const REASON: &str = "the file system does not say";

    pub(crate) fn new() -> Self {
        UnreadableInTheJob {
            answers: MemoryFreeSpace::unlimited(),
            thread: thread::current().id(),
        }
    }
}

impl FreeSpace for UnreadableInTheJob {
    fn free_space(&self, folder: &Path) -> io::Result<SpaceOnFileSystem> {
        if thread::current().id() == self.thread {
            self.answers.free_space(folder)
        } else {
            Err(io::Error::other(Self::REASON))
        }
    }
}
