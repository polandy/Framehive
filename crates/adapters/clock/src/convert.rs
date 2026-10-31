// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Conversion between the time types of `jiff` and those of `domain` (ADR 0022 §2).

use domain::{Timestamp, UtcOffset};
use jiff::tz::Offset;

const NANOSECONDS_PER_SECOND: i32 = 1_000_000_000;

/// Converts a `jiff` timestamp into a [`Timestamp`].
///
/// `jiff` gives the fraction of a second the sign of the seconds: 0.25 s before the
/// epoch is 0 s and -250 000 000 ns. A `Timestamp` counts nanoseconds forward from a
/// whole second: second -1 plus 750 000 000 ns.
pub(crate) fn to_timestamp(time: jiff::Timestamp) -> Timestamp {
    let mut seconds = time.as_second();
    let mut nanoseconds = time.subsec_nanosecond();
    if nanoseconds < 0 {
        seconds -= 1;
        nanoseconds += NANOSECONDS_PER_SECOND;
    }
    // `jiff` keeps the fraction within one second, so it now lies in 0..1e9.
    let nanoseconds = u32::try_from(nanoseconds).unwrap_or_else(|error| unreachable!("{error}"));
    Timestamp::new(seconds, nanoseconds).unwrap_or_else(|error| unreachable!("{error}"))
}

/// Converts a [`Timestamp`] into a `jiff` timestamp, to the whole second.
///
/// Time zones change their offset only at whole seconds, so the offset at the whole
/// second is the offset at the instant. `jiff` covers the years -9999 to 9999; a
/// timestamp beyond is clamped to that range.
pub(crate) fn to_jiff_timestamp(time: Timestamp) -> jiff::Timestamp {
    let seconds = time.unix_seconds();
    jiff::Timestamp::from_second(seconds).unwrap_or(if seconds < 0 {
        jiff::Timestamp::MIN
    } else {
        jiff::Timestamp::MAX
    })
}

/// Converts a `jiff` offset into a [`UtcOffset`].
///
/// `jiff` allows offsets up to 25:59:59, `UtcOffset` up to 23:59:59. No time zone has
/// ever used more than 15 hours, so a larger offset only comes from a broken `TZ`
/// value; local time is then UTC, as for an unknown time zone.
pub(crate) fn to_utc_offset(offset: Offset) -> UtcOffset {
    UtcOffset::from_seconds(offset.seconds()).unwrap_or(UtcOffset::UTC)
}
