// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use thiserror::Error;

use crate::{Timestamp, UtcOffset};

/// A calendar date and wall-clock time without a time zone, to the second.
///
/// Capture times are the wall-clock time the camera recorded, and the import time is
/// the machine's local time (IMPORT-001). Neither is converted to another time zone,
/// so this type deliberately carries none.
///
/// ```
/// use domain::LocalDateTime;
///
/// let time = LocalDateTime::new(2026, 10, 17, 14, 5, 9)?;
/// assert_eq!(time.day_of_year(), 290);
/// # Ok::<(), domain::DateTimeError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalDateTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

/// Error returned when the parts of a [`LocalDateTime`] do not form a valid time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{field} {value} is out of range")]
pub struct DateTimeError {
    /// The part that is out of range: `year`, `month`, `day`, `hour`, `minute` or
    /// `second`.
    pub field: &'static str,
    /// The rejected value.
    pub value: u16,
}

const SECONDS_PER_DAY: i64 = 86_400;

const DAYS_BEFORE_MONTH: [u16; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

impl LocalDateTime {
    /// Creates a date and time after checking every part.
    ///
    /// The year is 1–9999, so that `%Y` always has four digits. Leap years follow the
    /// Gregorian calendar; leap seconds are not accepted.
    ///
    /// # Errors
    ///
    /// Returns a [`DateTimeError`] naming the first part that is out of range.
    pub fn new(
        year: u16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<Self, DateTimeError> {
        check("year", year, 1, 9999)?;
        check("month", month.into(), 1, 12)?;
        check("day", day.into(), 1, days_in_month(year, month).into())?;
        check("hour", hour.into(), 0, 23)?;
        check("minute", minute.into(), 0, 59)?;
        check("second", second.into(), 0, 59)?;
        Ok(LocalDateTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }

    /// The wall-clock time at `instant` in a time zone that is `offset` from UTC.
    ///
    /// The fraction of a second is dropped. Returns `None` if the local time falls
    /// outside the years 1–9999.
    ///
    /// ```
    /// use domain::{LocalDateTime, Timestamp, UtcOffset};
    ///
    /// // 2026-07-01 12:00:00 UTC in Central European Summer Time.
    /// let instant = Timestamp::from_unix_seconds(1_782_907_200);
    /// let summer = UtcOffset::from_seconds(2 * 3_600)?;
    /// let local = LocalDateTime::from_timestamp(instant, summer);
    /// assert_eq!(local, Some(LocalDateTime::new(2026, 7, 1, 14, 0, 0)?));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn from_timestamp(instant: Timestamp, offset: UtcOffset) -> Option<Self> {
        let local_seconds = instant
            .unix_seconds()
            .checked_add(i64::from(offset.seconds()))?;
        let days = local_seconds.div_euclid(SECONDS_PER_DAY);
        let second_of_day = local_seconds.rem_euclid(SECONDS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        LocalDateTime::new(
            u16::try_from(year).ok()?,
            u8::try_from(month).ok()?,
            u8::try_from(day).ok()?,
            u8::try_from(second_of_day / 3_600).ok()?,
            u8::try_from(second_of_day % 3_600 / 60).ok()?,
            u8::try_from(second_of_day % 60).ok()?,
        )
        .ok()
    }

    /// The year, 1–9999.
    #[must_use]
    pub fn year(&self) -> u16 {
        self.year
    }

    /// The month, 1–12.
    #[must_use]
    pub fn month(&self) -> u8 {
        self.month
    }

    /// The day of the month, 1–31.
    #[must_use]
    pub fn day(&self) -> u8 {
        self.day
    }

    /// The hour, 0–23.
    #[must_use]
    pub fn hour(&self) -> u8 {
        self.hour
    }

    /// The minute, 0–59.
    #[must_use]
    pub fn minute(&self) -> u8 {
        self.minute
    }

    /// The second, 0–59.
    #[must_use]
    pub fn second(&self) -> u8 {
        self.second
    }

    /// The day of the year, 1–366.
    #[must_use]
    pub fn day_of_year(&self) -> u16 {
        let month_index = usize::from(self.month - 1);
        let leap_day = u16::from(self.month > 2 && is_leap_year(self.year));
        DAYS_BEFORE_MONTH[month_index] + leap_day + u16::from(self.day)
    }
}

/// The Gregorian year, month and day of the day `days` after 1970-01-01.
///
/// Howard Hinnant's `civil_from_days`: count in 400-year eras that start on March 1,
/// so the leap day is the last day of each year in the computation.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    const DAYS_PER_ERA: i64 = 146_097;
    const DAYS_FROM_ERA_START_TO_EPOCH: i64 = 719_468;
    let shifted = days.saturating_add(DAYS_FROM_ERA_START_TO_EPOCH);
    let era = shifted.div_euclid(DAYS_PER_ERA);
    let day_of_era = shifted.rem_euclid(DAYS_PER_ERA);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era.saturating_mul(400) + i64::from(month <= 2);
    (year, month, day)
}

fn check(field: &'static str, value: u16, min: u16, max: u16) -> Result<(), DateTimeError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(DateTimeError { field, value })
    }
}

fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Days in `month` of `year`; 0 for a month outside 1–12, which `new` rejects first.
fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Clock;
    use crate::testing::ManualClock;

    #[test]
    fn rejects_parts_out_of_range() {
        let rejected = |result: Result<LocalDateTime, DateTimeError>| result.unwrap_err().field;

        assert_eq!(rejected(LocalDateTime::new(0, 1, 1, 0, 0, 0)), "year");
        assert_eq!(rejected(LocalDateTime::new(2026, 13, 1, 0, 0, 0)), "month");
        assert_eq!(rejected(LocalDateTime::new(2026, 2, 29, 0, 0, 0)), "day");
        assert_eq!(rejected(LocalDateTime::new(2026, 1, 1, 24, 0, 0)), "hour");
        assert_eq!(rejected(LocalDateTime::new(2026, 1, 1, 0, 60, 0)), "minute");
        assert_eq!(rejected(LocalDateTime::new(2026, 1, 1, 0, 0, 60)), "second");
    }

    #[test]
    fn accepts_leap_days_by_the_gregorian_rule() {
        assert!(LocalDateTime::new(2024, 2, 29, 0, 0, 0).is_ok());
        assert!(LocalDateTime::new(2000, 2, 29, 0, 0, 0).is_ok());
        assert!(LocalDateTime::new(1900, 2, 29, 0, 0, 0).is_err());
    }

    #[test]
    fn counts_the_day_of_the_year() {
        let day = |month, day| LocalDateTime::new(2024, month, day, 0, 0, 0).unwrap();

        assert_eq!(day(1, 1).day_of_year(), 1);
        assert_eq!(day(3, 1).day_of_year(), 61);
        assert_eq!(day(12, 31).day_of_year(), 366);
    }

    /// spec: CORE-005.11
    #[test]
    fn modification_time_is_shown_with_the_local_offset_at_that_instant() {
        // Europe/Zurich: +02:00 in summer, +01:00 in winter, as the clock reports it.
        let clock = ManualClock::new(Timestamp::UNIX_EPOCH, UtcOffset::UTC);
        let local = |unix_seconds, offset_hours| {
            let modified = Timestamp::from_unix_seconds(unix_seconds);
            clock.set_local_offset(UtcOffset::from_seconds(offset_hours * 3_600).unwrap());
            LocalDateTime::from_timestamp(modified, clock.local_offset(modified)).unwrap()
        };

        // 2026-07-01 12:00:00 UTC and 2026-01-01 12:00:00 UTC.
        assert_eq!(
            local(1_782_907_200, 2),
            LocalDateTime::new(2026, 7, 1, 14, 0, 0).unwrap()
        );
        assert_eq!(
            local(1_767_268_800, 1),
            LocalDateTime::new(2026, 1, 1, 13, 0, 0).unwrap()
        );
    }

    #[test]
    fn from_timestamp_crosses_days_and_leap_days() {
        let at = |seconds, offset| {
            LocalDateTime::from_timestamp(
                Timestamp::from_unix_seconds(seconds),
                UtcOffset::from_seconds(offset).unwrap(),
            )
        };

        assert_eq!(at(0, 0), LocalDateTime::new(1970, 1, 1, 0, 0, 0).ok());
        assert_eq!(at(0, -1), LocalDateTime::new(1969, 12, 31, 23, 59, 59).ok());
        // 2024-02-29 23:30:00 UTC is already March 1 at +01:00.
        assert_eq!(
            at(1_709_249_400, 3_600),
            LocalDateTime::new(2024, 3, 1, 0, 30, 0).ok()
        );
        assert_eq!(
            at(951_782_400, 0),
            LocalDateTime::new(2000, 2, 29, 0, 0, 0).ok()
        );
        assert_eq!(
            at(253_402_300_799, 0),
            LocalDateTime::new(9999, 12, 31, 23, 59, 59).ok()
        );
    }

    #[test]
    fn from_timestamp_rejects_years_outside_the_range() {
        let at = |seconds| {
            LocalDateTime::from_timestamp(Timestamp::from_unix_seconds(seconds), UtcOffset::UTC)
        };

        assert_eq!(at(253_402_300_800), None);
        assert_eq!(at(-62_135_596_801), None);
        assert_eq!(
            at(-62_135_596_800),
            LocalDateTime::new(1, 1, 1, 0, 0, 0).ok()
        );
        assert_eq!(at(i64::MAX), None);
        assert_eq!(at(i64::MIN), None);
    }
}
