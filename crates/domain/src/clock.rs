// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{Timestamp, UtcOffset};

/// Port for the wall clock and the local time zone (ADR 0020, ADR 0022).
///
/// Domain logic and services never read the system time themselves; they ask a
/// `Clock`. `adapter-clock` implements it with the system time zone, and tests use
/// [`testing::ManualClock`](crate::testing::ManualClock).
pub trait Clock {
    /// Returns the current instant.
    fn now(&self) -> Timestamp;

    /// Returns the offset of local time from UTC at the instant `at`.
    ///
    /// The offset depends on the instant because of daylight saving time and changes
    /// of a time zone's rules.
    fn local_offset(&self, at: Timestamp) -> UtcOffset;
}
