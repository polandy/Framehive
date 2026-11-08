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
