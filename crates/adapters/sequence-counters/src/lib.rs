// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `sequence-counters` adapter of Framehive: it keeps the persisted sequence
//! counters in a text file of the state folder and implements the
//! [`naming::SequenceCounters`] port (ADR 0029). Only apps depend on it. See
//! `README.md`.

#![forbid(unsafe_code)]

mod format;
mod store;

pub use store::{COUNTER_FILE_NAME, FileSequenceCounters};
