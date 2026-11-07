// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The job queue of Framehive (CORE-007, ADR 0035).
//!
//! A [`JobQueue`] runs [`Job`]s in the background, one after another on a worker
//! thread of its own, and reports each change of a job's [`JobState`] over a channel.
//! It knows nothing about what a job does.

#![forbid(unsafe_code)]

mod job;
mod queue;
mod worker;

pub use job::{Job, JobControl, JobEnd, JobId, JobState, JobUpdate};
pub use queue::{JobQueue, NotQueuedOrRunning};
