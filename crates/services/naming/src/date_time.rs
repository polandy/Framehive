// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use thiserror::Error;

/// A calendar date and wall-clock time without a time zone, to the second.
///
/// Capture times are the wall-clock time the camera recorded, and the import time is
/// the machine's local time (IMPORT-001). Neither is converted to another time zone,
/// so this type deliberately carries none.
///
/// ```
/// use naming::LocalDateTime;
///
/// let time = LocalDateTime::new(2026, 10, 17, 14, 5, 9)?;
/// assert_eq!(time.day_of_year(), 290);
/// # Ok::<(), naming::DateTimeError>(())
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
}
