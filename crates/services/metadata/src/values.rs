// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The text values of `DateTimeOriginal`, `SubSecTimeOriginal` and
//! `OffsetTimeOriginal` (CIPA DC-008), turned into a [`CaptureTime`].

use domain::{CaptureTime, LocalDateTime, UtcOffset};

use crate::NoCaptureTime;

const NANOSECOND_DIGITS: usize = 9;
const SECONDS_PER_HOUR: i32 = 3_600;
const SECONDS_PER_MINUTE: i32 = 60;

/// The capture time from the raw tag values. Invalid sub-seconds or offsets are
/// ignored (CORE-005); only the date and time decide whether there is one.
pub(crate) fn capture_time(
    date_time: &[u8],
    sub_seconds: Option<&[u8]>,
    offset: Option<&[u8]>,
) -> Result<CaptureTime, NoCaptureTime> {
    let mut time = CaptureTime::new(local_date_time(text(date_time))?);
    if let Some(nanoseconds) = sub_seconds.and_then(|value| nanoseconds(text(value))) {
        // `nanoseconds` stays below one second, so this cannot fail.
        time = time.with_subsec_nanoseconds(nanoseconds).unwrap_or(time);
    }
    if let Some(offset) = offset.and_then(|value| utc_offset(text(value))) {
        time = time.with_offset(offset);
    }
    Ok(time)
}

/// An EXIF ASCII value up to its terminating NUL.
fn text(value: &[u8]) -> &[u8] {
    value.split(|byte| *byte == 0).next().unwrap_or_default()
}

/// Parses `YYYY:MM:DD HH:MM:SS`. EXIF writes an unknown time as blanks or zeros,
/// keeping or dropping the colons.
fn local_date_time(value: &[u8]) -> Result<LocalDateTime, NoCaptureTime> {
    if value.iter().all(|byte| matches!(byte, b' ' | b':' | b'0')) {
        return Err(NoCaptureTime::Missing);
    }
    let invalid = || NoCaptureTime::Invalid {
        value: String::from_utf8_lossy(value).escape_debug().to_string(),
    };
    let separators_match = value.len() == 19
        && [(4, b':'), (7, b':'), (10, b' '), (13, b':'), (16, b':')]
            .iter()
            .all(|&(index, separator)| value[index] == separator);
    if !separators_match {
        return Err(invalid());
    }
    let number = |start: usize, end: usize| digits(&value[start..end]).ok_or_else(invalid);
    let part = |start: usize| u8::try_from(number(start, start + 2)?).map_err(|_| invalid());
    let year = u16::try_from(number(0, 4)?).map_err(|_| invalid())?;
    LocalDateTime::new(year, part(5)?, part(8)?, part(11)?, part(14)?, part(17)?)
        .map_err(|_| invalid())
}

/// `SubSecTimeOriginal`: the digits of the fraction of a second, e.g. `42` for 0.42.
/// Digits beyond nanoseconds are dropped; trailing blanks are allowed.
fn nanoseconds(value: &[u8]) -> Option<u32> {
    let value = value.trim_ascii_end();
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mut padded = [b'0'; NANOSECOND_DIGITS];
    let used = value.len().min(NANOSECOND_DIGITS);
    padded[..used].copy_from_slice(&value[..used]);
    digits(&padded)
}

/// `OffsetTimeOriginal`: `+HH:MM` or `-HH:MM`, less than a day from UTC.
fn utc_offset(value: &[u8]) -> Option<UtcOffset> {
    let [sign, h1, h2, b':', m1, m2] = *value else {
        return None;
    };
    let hours = i32::try_from(digits(&[h1, h2])?).ok()?;
    let minutes = i32::try_from(digits(&[m1, m2])?).ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    let seconds = hours * SECONDS_PER_HOUR + minutes * SECONDS_PER_MINUTE;
    match sign {
        b'+' => UtcOffset::from_seconds(seconds).ok(),
        b'-' => UtcOffset::from_seconds(-seconds).ok(),
        _ => None,
    }
}

/// The number written by ASCII digits only, at most nine of them.
fn digits(value: &[u8]) -> Option<u32> {
    if value.is_empty() || value.len() > NANOSECOND_DIGITS {
        return None;
    }
    value.iter().try_fold(0_u32, |number, byte| {
        byte.is_ascii_digit()
            .then(|| number * 10 + u32::from(byte - b'0'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> Result<LocalDateTime, NoCaptureTime> {
        local_date_time(text.as_bytes())
    }

    #[test]
    fn reads_the_date_and_time() {
        assert_eq!(
            date("2026:10:17 14:05:09"),
            Ok(LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap())
        );
    }

    #[test]
    fn blanks_and_zeros_mean_unknown() {
        for unknown in ["", "0000:00:00 00:00:00", "    :  :     :  :  ", "   "] {
            assert_eq!(date(unknown), Err(NoCaptureTime::Missing), "{unknown:?}");
        }
    }

    #[test]
    fn rejects_other_layouts_and_out_of_range_parts() {
        for invalid in [
            "2026-10-17 14:05:09",
            "2026:10:17 14:05",
            "2026:10:17T14:05:09",
            "2026:1a:17 14:05:09",
            "2026:+1:17 14:05:09",
            "2026:02:29 14:05:09",
        ] {
            assert_eq!(
                date(invalid),
                Err(NoCaptureTime::Invalid {
                    value: invalid.to_owned()
                })
            );
        }
    }

    #[test]
    fn escapes_control_characters_in_an_invalid_value() {
        assert_eq!(
            date("2026:10:17\n14:05:09"),
            Err(NoCaptureTime::Invalid {
                value: "2026:10:17\\n14:05:09".to_owned()
            })
        );
    }

    #[test]
    fn sub_seconds_are_a_decimal_fraction() {
        assert_eq!(nanoseconds(b"42"), Some(420_000_000));
        assert_eq!(nanoseconds(b"007 "), Some(7_000_000));
        assert_eq!(nanoseconds(b"1234567891"), Some(123_456_789));
        assert_eq!(nanoseconds(b""), None);
        assert_eq!(nanoseconds(b"4x"), None);
        assert_eq!(nanoseconds(b" 4"), None);
    }

    #[test]
    fn offsets_are_signed_hours_and_minutes() {
        let offset = |text: &str| utc_offset(text.as_bytes()).map(UtcOffset::seconds);
        assert_eq!(offset("+02:00"), Some(7_200));
        assert_eq!(offset("-03:30"), Some(-12_600));
        assert_eq!(offset("+23:59"), Some(86_340));
        assert_eq!(offset("+25:00"), None);
        assert_eq!(offset("+02:60"), None);
        assert_eq!(offset("02:00"), None);
        assert_eq!(offset("   :  "), None);
        assert_eq!(offset("*02:00"), None);
    }

    #[test]
    fn values_end_at_the_first_nul() {
        let time = capture_time(
            b"2026:10:17 14:05:09\0junk",
            Some(b"5\0"),
            Some(b"+01:00\0"),
        );
        let time = time.unwrap();
        assert_eq!(time.subsec_nanoseconds(), Some(500_000_000));
        assert_eq!(time.offset().map(UtcOffset::seconds), Some(3_600));
    }
}
