// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `clock` adapter of Framehive: it implements the [`domain::Clock`] port with
//! the system clock and the system time zone (ADR 0020 §4, ADR 0022). Only apps
//! depend on it, and no other crate sees a `jiff` type. See `README.md`.
//!
//! ```no_run
//! use adapter_clock::SystemClock;
//! use domain::Clock;
//!
//! let clock = SystemClock::new();
//! let now = clock.now();
//! let offset = clock.local_offset(now);
//! assert!(offset.seconds().abs() < 24 * 3_600);
//! ```

#![forbid(unsafe_code)]

mod convert;

use domain::{Clock, Timestamp, UtcOffset};
use jiff::tz::TimeZone;

/// The [`Clock`] of the machine: the system time, and local time in the system time
/// zone.
///
/// The time zone is read once, by [`SystemClock::new`], from `TZ`, `/etc/localtime`
/// and the system time zone database. If it cannot be found, local time is UTC.
#[derive(Debug, Clone)]
pub struct SystemClock {
    time_zone: TimeZone,
}

impl SystemClock {
    /// Creates the clock with the system time zone.
    #[must_use]
    pub fn new() -> Self {
        SystemClock::with_time_zone(TimeZone::system())
    }

    /// Creates the clock with `time_zone`, so tests need not depend on the machine's
    /// time zone.
    fn with_time_zone(time_zone: TimeZone) -> Self {
        SystemClock { time_zone }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        SystemClock::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        convert::to_timestamp(jiff::Timestamp::now())
    }

    fn local_offset(&self, at: Timestamp) -> UtcOffset {
        let offset = self.time_zone.to_offset(convert::to_jiff_timestamp(at));
        convert::to_utc_offset(offset)
    }
}

#[cfg(test)]
mod tests;
