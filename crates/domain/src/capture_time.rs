// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{LocalDateTime, TimeError, UtcOffset};

const NANOSECONDS_PER_SECOND: u32 = 1_000_000_000;

/// The wall-clock time the camera recorded (glossary: Capture time, CORE-005).
///
/// It comes from EXIF `DateTimeOriginal`, with the fraction of a second from
/// `SubSecTimeOriginal` and the UTC offset from `OffsetTimeOriginal` when the camera
/// wrote them. It is never converted to another time zone: names and folders use
/// [`CaptureTime::date_time`] as recorded.
///
/// ```
/// use domain::{CaptureTime, LocalDateTime, UtcOffset};
///
/// let recorded = LocalDateTime::new(2026, 10, 17, 14, 5, 9)?;
/// let capture_time = CaptureTime::new(recorded)
///     .with_subsec_nanoseconds(420_000_000)?
///     .with_offset(UtcOffset::from_seconds(2 * 3_600)?);
///
/// assert_eq!(capture_time.date_time(), recorded);
/// assert_eq!(capture_time.subsec_nanoseconds(), Some(420_000_000));
/// assert_eq!(capture_time.offset().map(|offset| offset.to_string()).as_deref(), Some("+02:00"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CaptureTime {
    date_time: LocalDateTime,
    subsec_nanoseconds: Option<u32>,
    offset: Option<UtcOffset>,
}

impl CaptureTime {
    /// A capture time to the second, without sub-seconds and without an offset.
    #[must_use]
    pub fn new(date_time: LocalDateTime) -> Self {
        CaptureTime {
            date_time,
            subsec_nanoseconds: None,
            offset: None,
        }
    }

    /// The same capture time with the fraction of a second, in nanoseconds.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NanosecondsOutOfRange`] if `nanoseconds` is one second or
    /// more.
    pub fn with_subsec_nanoseconds(self, nanoseconds: u32) -> Result<Self, TimeError> {
        if nanoseconds >= NANOSECONDS_PER_SECOND {
            return Err(TimeError::NanosecondsOutOfRange { nanoseconds });
        }
        Ok(CaptureTime {
            subsec_nanoseconds: Some(nanoseconds),
            ..self
        })
    }

    /// The same capture time with the UTC offset the camera recorded.
    #[must_use]
    pub fn with_offset(self, offset: UtcOffset) -> Self {
        CaptureTime {
            offset: Some(offset),
            ..self
        }
    }

    /// The recorded date and wall-clock time, to the second.
    #[must_use]
    pub fn date_time(&self) -> LocalDateTime {
        self.date_time
    }

    /// The fraction of a second in nanoseconds, if the camera recorded one.
    #[must_use]
    pub fn subsec_nanoseconds(&self) -> Option<u32> {
        self.subsec_nanoseconds
    }

    /// The offset of the recorded time from UTC, if the camera recorded one.
    #[must_use]
    pub fn offset(&self) -> Option<UtcOffset> {
        self.offset
    }
}

impl From<LocalDateTime> for CaptureTime {
    fn from(date_time: LocalDateTime) -> Self {
        CaptureTime::new(date_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_full_second_of_sub_seconds() {
        let time = CaptureTime::new(LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap());
        assert_eq!(
            time.with_subsec_nanoseconds(1_000_000_000),
            Err(TimeError::NanosecondsOutOfRange {
                nanoseconds: 1_000_000_000
            })
        );
        assert_eq!(
            time.with_subsec_nanoseconds(999_999_999)
                .unwrap()
                .subsec_nanoseconds(),
            Some(999_999_999)
        );
    }

    #[test]
    fn starts_without_sub_seconds_and_offset() {
        let recorded = LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap();
        let time = CaptureTime::from(recorded);
        assert_eq!(time.date_time(), recorded);
        assert_eq!(time.subsec_nanoseconds(), None);
        assert_eq!(time.offset(), None);
    }
}
