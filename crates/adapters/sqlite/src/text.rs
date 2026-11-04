// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The text of the time columns (ADR 0031, section 3): capture times as
//! `YYYY-MM-DDTHH:MM:SS`, recorded times as RFC 3339 `YYYY-MM-DDTHH:MM:SS±HH:MM`.

use std::fmt;

use domain::{LocalDateTime, UtcOffset};
use history::RecordedTime;

/// A time column whose text is not in the format of ADR 0031, section 3.
#[derive(Debug)]
pub(crate) struct InvalidTime(pub(crate) String);

impl fmt::Display for InvalidTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid recorded time {:?}", self.0)
    }
}

impl std::error::Error for InvalidTime {}

/// A capture time as `YYYY-MM-DDTHH:MM:SS`.
pub(crate) fn capture_time_text(time: LocalDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        time.year(),
        time.month(),
        time.day(),
        time.hour(),
        time.minute(),
        time.second()
    )
}

/// Reads a capture time written by [`capture_time_text`].
pub(crate) fn parse_capture_time(text: &str) -> Result<LocalDateTime, InvalidTime> {
    parse_local(text).ok_or_else(|| InvalidTime(text.to_owned()))
}

/// Reads a recorded time written by `RecordedTime`'s `Display`.
pub(crate) fn parse_recorded_time(text: &str) -> Result<RecordedTime, InvalidTime> {
    let invalid = || InvalidTime(text.to_owned());
    let (local, offset) = text.split_at_checked(19).ok_or_else(invalid)?;
    let local = parse_local(local).ok_or_else(invalid)?;
    let offset = parse_offset(offset).ok_or_else(invalid)?;
    Ok(RecordedTime { local, offset })
}

/// `YYYY-MM-DDTHH:MM:SS`, with exactly these separators.
fn parse_local(text: &str) -> Option<LocalDateTime> {
    let bytes = text.as_bytes();
    let separators = [(4, b'-'), (7, b'-'), (10, b'T'), (13, b':'), (16, b':')];
    if bytes.len() != 19 || separators.iter().any(|&(at, byte)| bytes[at] != byte) {
        return None;
    }
    LocalDateTime::new(
        number(text, 0..4)?,
        number(text, 5..7)?,
        number(text, 8..10)?,
        number(text, 11..13)?,
        number(text, 14..16)?,
        number(text, 17..19)?,
    )
    .ok()
}

/// `±HH:MM`.
fn parse_offset(text: &str) -> Option<UtcOffset> {
    let bytes = text.as_bytes();
    if bytes.len() != 6 || bytes[3] != b':' {
        return None;
    }
    let sign = match bytes[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hours: i32 = number(text, 1..3)?;
    let minutes: i32 = number(text, 4..6)?;
    UtcOffset::from_seconds(sign * (hours * 3_600 + minutes * 60)).ok()
}

/// The decimal digits of `text` in `range`, and nothing else.
fn number<T: std::str::FromStr>(text: &str, range: std::ops::Range<usize>) -> Option<T> {
    let digits = text.get(range)?;
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

    use super::*;
    use domain::Timestamp;

    #[test]
    fn a_recorded_time_reads_back_as_written() {
        for seconds in [7_200, -34_200, 0] {
            let offset = UtcOffset::from_seconds(seconds).unwrap();
            let time =
                RecordedTime::at(Timestamp::from_unix_seconds(1_793_437_200), offset).unwrap();

            assert_eq!(parse_recorded_time(&time.to_string()).unwrap(), time);
        }
    }

    #[test]
    fn a_capture_time_reads_back_as_written() {
        let time = LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap();

        assert_eq!(parse_capture_time(&capture_time_text(time)).unwrap(), time);
    }

    #[test]
    fn text_in_another_format_is_invalid() {
        for text in [
            "",
            "2026-10-17 14:05:09+02:00",
            "2026-10-17T14:05:09",
            "2026-10-17T14:05:09Z",
            "2026-10-17T14:05:09+0200",
            "2026-13-17T14:05:09+02:00",
            "2026-10-17T14:05:+9+02:00",
        ] {
            assert!(parse_recorded_time(text).is_err(), "{text}");
        }
    }
}
