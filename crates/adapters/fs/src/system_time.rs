// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Conversion between [`SystemTime`], the time `std::fs` speaks, and [`Timestamp`].

use std::io;
use std::time::{Duration, SystemTime};

use domain::Timestamp;

const NANOSECONDS_PER_SECOND: u32 = 1_000_000_000;

/// Converts a time from the file system into a [`Timestamp`].
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidData`] if the time lies too far from the epoch
/// for a `Timestamp`.
pub(crate) fn to_timestamp(time: SystemTime) -> io::Result<Timestamp> {
    let (seconds, nanoseconds) = match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(after) => (seconds_of(after)?, after.subsec_nanos()),
        Err(before) => before_epoch(before.duration())?,
    };
    Timestamp::new(seconds, nanoseconds).map_err(|error| io::Error::other(error.to_string()))
}

/// A `Timestamp` counts nanoseconds forward from a whole second, also before the
/// epoch: 0.25 s before it is second -1 plus 750 000 000 ns.
fn before_epoch(before: Duration) -> io::Result<(i64, u32)> {
    let seconds = -seconds_of(before)?;
    match before.subsec_nanos() {
        0 => Ok((seconds, 0)),
        nanoseconds => Ok((seconds - 1, NANOSECONDS_PER_SECOND - nanoseconds)),
    }
}

fn seconds_of(duration: Duration) -> io::Result<i64> {
    i64::try_from(duration.as_secs()).map_err(|_| out_of_range(io::ErrorKind::InvalidData))
}

/// Converts a [`Timestamp`] into a time `std::fs` can store.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidInput`] if the platform cannot represent the time.
pub(crate) fn to_system_time(time: Timestamp) -> io::Result<SystemTime> {
    let whole_seconds = Duration::from_secs(time.unix_seconds().unsigned_abs());
    let whole = if time.unix_seconds() >= 0 {
        SystemTime::UNIX_EPOCH.checked_add(whole_seconds)
    } else {
        SystemTime::UNIX_EPOCH.checked_sub(whole_seconds)
    };
    whole
        .and_then(|whole| {
            whole.checked_add(Duration::from_nanos(u64::from(time.subsec_nanoseconds())))
        })
        .ok_or_else(|| out_of_range(io::ErrorKind::InvalidInput))
}

fn out_of_range(kind: io::ErrorKind) -> io::Error {
    io::Error::new(kind, "the time lies outside the range of the file system")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(seconds: i64, nanoseconds: u32) -> Timestamp {
        Timestamp::new(seconds, nanoseconds).unwrap()
    }

    #[test]
    fn converts_times_after_the_epoch_with_nanoseconds() {
        let time = SystemTime::UNIX_EPOCH + Duration::new(1_792_245_908, 123_456_789);
        assert_eq!(
            to_timestamp(time).unwrap(),
            timestamp(1_792_245_908, 123_456_789)
        );
    }

    #[test]
    fn converts_times_before_the_epoch_counting_nanoseconds_forward() {
        let quarter_before = SystemTime::UNIX_EPOCH - Duration::from_millis(250);
        assert_eq!(
            to_timestamp(quarter_before).unwrap(),
            timestamp(-1, 750_000_000)
        );
        let whole_before = SystemTime::UNIX_EPOCH - Duration::from_secs(86_401);
        assert_eq!(to_timestamp(whole_before).unwrap(), timestamp(-86_401, 0));
    }

    #[test]
    fn round_trips_through_system_time() {
        for time in [
            Timestamp::UNIX_EPOCH,
            timestamp(1_792_245_908, 1),
            timestamp(-1, 750_000_000),
            timestamp(-2_208_988_800, 999_999_999),
        ] {
            assert_eq!(to_timestamp(to_system_time(time).unwrap()).unwrap(), time);
        }
    }
}
