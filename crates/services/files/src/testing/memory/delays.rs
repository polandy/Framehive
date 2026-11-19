// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Calls of a [`MemoryFileSystem`](super::MemoryFileSystem) that hang or take time
//! (ADR 0049 §4).

use std::sync::Arc;
use std::time::Duration;

use crate::AnswerSignal;
use crate::testing::ManualTimer;

/// A hang that [`Fault::Hangs`](super::Fault::Hangs) injects: the call signals that it
/// has begun, then blocks until the test releases it. Clones share one hang.
///
/// A test releases the hang at its end, so the thread blocked in the call can end.
#[derive(Debug, Clone, Default)]
pub struct Hang {
    begun: AnswerSignal,
    released: AnswerSignal,
}

impl Hang {
    /// A hang that no call has reached yet.
    #[must_use]
    pub fn new() -> Self {
        Hang::default()
    }

    /// Blocks until a call has begun to hang.
    pub fn wait_until_begun(&self) {
        self.begun.wait_until(|| false);
    }

    /// Lets every call that hangs, and every later one, answer as usual.
    pub fn release(&self) {
        self.released.signal();
    }

    pub(super) fn block(&self) {
        self.begun.signal();
        self.released.wait_until(|| false);
    }
}

impl PartialEq for Hang {
    fn eq(&self, other: &Self) -> bool {
        self.begun.is_same_as(&other.begun)
    }
}

impl Eq for Hang {}

/// Reads that take time, which [`Fault::SlowReads`](super::Fault::SlowReads) injects:
/// each read advances a [`ManualTimer`] by `each_read` before it answers.
#[derive(Debug, Clone)]
pub struct SlowReads {
    timer: Arc<ManualTimer>,
    each_read: Duration,
}

impl SlowReads {
    /// Reads that each let `each_read` pass on `timer`.
    #[must_use]
    pub fn new(timer: Arc<ManualTimer>, each_read: Duration) -> Self {
        SlowReads { timer, each_read }
    }

    pub(super) fn pass_time(&self) {
        self.timer.advance(self.each_read);
    }
}

impl PartialEq for SlowReads {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.timer, &other.timer) && self.each_read == other.each_read
    }
}

impl Eq for SlowReads {}
