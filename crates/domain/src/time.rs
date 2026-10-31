// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Instants and UTC offsets, std-only (ADR 0022). Calendar dates and wall-clock times
//! are `LocalDateTime` and `CaptureTime` (ADR 0021).

use std::fmt;
use std::time::Duration;

use thiserror::Error;

const NANOSECONDS_PER_SECOND: u32 = 1_000_000_000;
const SECONDS_PER_MINUTE: i32 = 60;
const SECONDS_PER_HOUR: i32 = 3_600;

/// An instant in UTC, to the nanosecond, counted from the Unix epoch
/// (1970-01-01 00:00:00 UTC).
///
/// It has no time zone and no calendar; a [`UtcOffset`] turns it into local time.
///
/// ```
/// use domain::Timestamp;
///
/// // 2026-10-17 14:05:09 UTC
/// let modified = Timestamp::from_unix_seconds(1_792_245_909);
/// assert_eq!(modified.unix_seconds(), 1_792_245_909);
/// assert!(Timestamp::UNIX_EPOCH < modified);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    seconds: i64,
    nanoseconds: u32,
}

/// Error returned when values do not form a valid [`Timestamp`] or [`UtcOffset`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TimeError {
    /// The fraction of a second is one second or more.
    #[error("the nanoseconds of a timestamp must be below one second, got {nanoseconds}")]
    NanosecondsOutOfRange {
        /// The rejected number of nanoseconds.
        nanoseconds: u32,
    },
    /// A UTC offset is a day or more away from UTC.
    #[error("a UTC offset must be less than 24 hours from UTC, got {seconds} seconds")]
    OffsetOutOfRange {
        /// The rejected offset in seconds.
        seconds: i32,
    },
}

impl Timestamp {
    /// The Unix epoch, 1970-01-01 00:00:00 UTC.
    pub const UNIX_EPOCH: Timestamp = Timestamp {
        seconds: 0,
        nanoseconds: 0,
    };

    /// Creates a timestamp from whole seconds since the Unix epoch.
    #[must_use]
    pub fn from_unix_seconds(seconds: i64) -> Self {
        Timestamp {
            seconds,
            nanoseconds: 0,
        }
    }

    /// Creates a timestamp from seconds since the Unix epoch and the nanoseconds of
    /// the following fraction of a second.
    ///
    /// Before the epoch, `seconds` is negative and `nanoseconds` still counts forward:
    /// half a second before the epoch is `(-1, 500_000_000)`.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NanosecondsOutOfRange`] if `nanoseconds` is one second or
    /// more.
    pub fn new(seconds: i64, nanoseconds: u32) -> Result<Self, TimeError> {
        if nanoseconds >= NANOSECONDS_PER_SECOND {
            return Err(TimeError::NanosecondsOutOfRange { nanoseconds });
        }
        Ok(Timestamp {
            seconds,
            nanoseconds,
        })
    }

    /// Returns the whole seconds since the Unix epoch, rounded towards the past.
    #[must_use]
    pub fn unix_seconds(self) -> i64 {
        self.seconds
    }

    /// Returns the nanoseconds after [`Timestamp::unix_seconds`], below one second.
    #[must_use]
    pub fn subsec_nanoseconds(self) -> u32 {
        self.nanoseconds
    }

    /// Returns the timestamp `duration` later, or `None` if it cannot be represented.
    #[must_use]
    pub fn checked_add(self, duration: Duration) -> Option<Self> {
        let whole_seconds = i64::try_from(duration.as_secs()).ok()?;
        let mut seconds = self.seconds.checked_add(whole_seconds)?;
        let mut nanoseconds = self.nanoseconds + duration.subsec_nanos();
        if nanoseconds >= NANOSECONDS_PER_SECOND {
            nanoseconds -= NANOSECONDS_PER_SECOND;
            seconds = seconds.checked_add(1)?;
        }
        Some(Timestamp {
            seconds,
            nanoseconds,
        })
    }
}

/// The difference between local time and UTC, in seconds east of UTC.
///
/// ```
/// use domain::UtcOffset;
///
/// let berlin_summer = UtcOffset::from_seconds(2 * 3_600)?;
/// assert_eq!(berlin_summer.to_string(), "+02:00");
/// assert_eq!(UtcOffset::UTC.to_string(), "+00:00");
/// # Ok::<(), domain::TimeError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UtcOffset(i32);

impl UtcOffset {
    /// UTC itself.
    pub const UTC: UtcOffset = UtcOffset(0);

    /// The largest offset in seconds; offsets stay below a full day, as in the TC39
    /// Temporal proposal that the time library follows (ADR 0022).
    pub const MAX_SECONDS: i32 = 24 * SECONDS_PER_HOUR - 1;

    /// Creates an offset of `seconds` east of UTC; negative values are west of UTC.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::OffsetOutOfRange`] if the offset is a full day or more.
    pub fn from_seconds(seconds: i32) -> Result<Self, TimeError> {
        if seconds.unsigned_abs() > Self::MAX_SECONDS.unsigned_abs() {
            return Err(TimeError::OffsetOutOfRange { seconds });
        }
        Ok(UtcOffset(seconds))
    }

    /// Returns the offset in seconds east of UTC.
    #[must_use]
    pub fn seconds(self) -> i32 {
        self.0
    }
}

impl fmt::Display for UtcOffset {
    /// Writes `±HH:MM`, and `±HH:MM:SS` when the offset has seconds.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { '-' } else { '+' };
        let total = self.0.abs();
        let hours = total / SECONDS_PER_HOUR;
        let minutes = total % SECONDS_PER_HOUR / SECONDS_PER_MINUTE;
        let seconds = total % SECONDS_PER_MINUTE;
        write!(formatter, "{sign}{hours:02}:{minutes:02}")?;
        if seconds != 0 {
            write!(formatter, ":{seconds:02}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_rejects_a_full_second_of_nanoseconds() {
        assert_eq!(
            Timestamp::new(0, 1_000_000_000),
            Err(TimeError::NanosecondsOutOfRange {
                nanoseconds: 1_000_000_000
            })
        );
        let latest = Timestamp::new(5, 999_999_999).unwrap();
        assert_eq!(latest.unix_seconds(), 5);
        assert_eq!(latest.subsec_nanoseconds(), 999_999_999);
    }

    #[test]
    fn timestamps_order_by_seconds_then_nanoseconds() {
        let before_epoch = Timestamp::new(-1, 500_000_000).unwrap();
        let just_after = Timestamp::new(0, 1).unwrap();
        assert!(before_epoch < Timestamp::UNIX_EPOCH);
        assert!(Timestamp::UNIX_EPOCH < just_after);
        assert!(just_after < Timestamp::from_unix_seconds(1));
    }

    #[test]
    fn checked_add_carries_nanoseconds_into_seconds() {
        let start = Timestamp::new(10, 700_000_000).unwrap();
        let later = start.checked_add(Duration::new(1, 400_000_000)).unwrap();
        assert_eq!(later, Timestamp::new(12, 100_000_000).unwrap());
    }

    #[test]
    fn checked_add_reports_overflow() {
        let last = Timestamp::new(i64::MAX, 999_999_999).unwrap();
        assert_eq!(last.checked_add(Duration::from_nanos(1)), None);
        assert_eq!(
            Timestamp::UNIX_EPOCH.checked_add(Duration::from_secs(u64::MAX)),
            None
        );
    }

    #[test]
    fn utc_offset_stays_below_a_day() {
        assert_eq!(UtcOffset::from_seconds(86_399).unwrap().seconds(), 86_399);
        assert_eq!(UtcOffset::from_seconds(-86_399).unwrap().seconds(), -86_399);
        assert_eq!(
            UtcOffset::from_seconds(86_400),
            Err(TimeError::OffsetOutOfRange { seconds: 86_400 })
        );
        assert_eq!(
            UtcOffset::from_seconds(i32::MIN),
            Err(TimeError::OffsetOutOfRange { seconds: i32::MIN })
        );
    }

    #[test]
    fn utc_offset_is_written_with_sign_hours_and_minutes() {
        let written = |seconds| UtcOffset::from_seconds(seconds).unwrap().to_string();
        assert_eq!(written(0), "+00:00");
        assert_eq!(written(19_800), "+05:30");
        assert_eq!(written(-12_600), "-03:30");
        assert_eq!(written(-(10 * 3_600 + 29 * 60 + 20)), "-10:29:20");
    }
}
