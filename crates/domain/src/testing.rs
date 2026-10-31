// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Test doubles for the ports of this crate.

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use crate::{Clock, Timestamp, UtcOffset};

/// A [`Clock`] that shows the time it is told, for deterministic tests.
///
/// ```
/// use std::time::Duration;
///
/// use domain::testing::ManualClock;
/// use domain::{Clock, Timestamp, UtcOffset};
///
/// let clock = ManualClock::new(Timestamp::from_unix_seconds(100), UtcOffset::UTC);
/// clock.advance(Duration::from_secs(5));
/// assert_eq!(clock.now(), Timestamp::from_unix_seconds(105));
/// ```
#[derive(Debug)]
pub struct ManualClock {
    state: Mutex<ClockState>,
}

#[derive(Debug, Clone, Copy)]
struct ClockState {
    now: Timestamp,
    local_offset: UtcOffset,
}

impl ManualClock {
    /// Creates a clock that shows `now`, with the same local offset at every instant.
    #[must_use]
    pub fn new(now: Timestamp, local_offset: UtcOffset) -> Self {
        ManualClock {
            state: Mutex::new(ClockState { now, local_offset }),
        }
    }

    /// Sets the current instant.
    pub fn set_now(&self, now: Timestamp) {
        self.update(|state| state.now = now);
    }

    /// Moves the current instant forward by `duration`, stopping at the latest
    /// instant that can be represented.
    pub fn advance(&self, duration: Duration) {
        self.update(|state| {
            if let Some(later) = state.now.checked_add(duration) {
                state.now = later;
            }
        });
    }

    /// Sets the local offset reported for every instant.
    pub fn set_local_offset(&self, local_offset: UtcOffset) {
        self.update(|state| state.local_offset = local_offset);
    }

    fn update(&self, change: impl FnOnce(&mut ClockState)) {
        // A poisoned lock only means another test thread panicked; the state is plain
        // data and still valid.
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        change(&mut state);
    }

    fn snapshot(&self) -> ClockState {
        *self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Timestamp {
        self.snapshot().now
    }

    fn local_offset(&self, _at: Timestamp) -> UtcOffset {
        self.snapshot().local_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_the_time_it_is_set_to() {
        let clock = ManualClock::new(Timestamp::from_unix_seconds(1), UtcOffset::UTC);
        assert_eq!(clock.now(), Timestamp::from_unix_seconds(1));

        clock.set_now(Timestamp::from_unix_seconds(1_792_245_909));
        assert_eq!(clock.now(), Timestamp::from_unix_seconds(1_792_245_909));
    }

    #[test]
    fn advances_by_a_duration() {
        let clock = ManualClock::new(Timestamp::UNIX_EPOCH, UtcOffset::UTC);
        clock.advance(Duration::from_millis(1_500));
        assert_eq!(clock.now(), Timestamp::new(1, 500_000_000).unwrap());
    }

    #[test]
    fn stops_at_the_latest_instant() {
        let last = Timestamp::new(i64::MAX, 999_999_999).unwrap();
        let clock = ManualClock::new(last, UtcOffset::UTC);
        clock.advance(Duration::from_secs(1));
        assert_eq!(clock.now(), last);
    }

    #[test]
    fn reports_the_local_offset_it_is_set_to() {
        let summer = UtcOffset::from_seconds(7_200).unwrap();
        let clock = ManualClock::new(Timestamp::UNIX_EPOCH, summer);
        assert_eq!(clock.local_offset(Timestamp::UNIX_EPOCH), summer);

        clock.set_local_offset(UtcOffset::UTC);
        assert_eq!(clock.local_offset(clock.now()), UtcOffset::UTC);
    }
}
