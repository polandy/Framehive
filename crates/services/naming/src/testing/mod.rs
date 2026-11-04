// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The test double and the conformance suite of the
//! [`SequenceCounters`](crate::SequenceCounters) port (ADR 0029).
//!
//! `adapter-sequence-counters` runs [`check_sequence_counters`] against a temporary
//! state folder.

mod conformance;
mod memory;

pub use conformance::{SequenceCountersUnderTest, check_sequence_counters};
pub use memory::MemorySequenceCounters;
