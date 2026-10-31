// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The clock with a fixed time zone from a POSIX `TZ` rule, which needs no time zone
//! database, and the conversions between `jiff` and `domain`. Nothing here reads the
//! wall clock or the machine's time zone (CLAUDE.md rule 4).

use domain::{Clock, Timestamp, UtcOffset};
use jiff::tz::{Offset, TimeZone};

use super::SystemClock;
use super::convert::{to_jiff_timestamp, to_timestamp, to_utc_offset};

/// Central European Time: UTC+1, and UTC+2 from the last Sunday of March, 02:00, to
/// the last Sunday of October, 03:00.
const CENTRAL_EUROPE: &str = "CET-1CEST,M3.5.0,M10.5.0/3";
const HOUR: i32 = 3_600;

fn central_europe() -> SystemClock {
    SystemClock::with_time_zone(TimeZone::posix(CENTRAL_EUROPE).unwrap())
}

fn offset_at(seconds: i64) -> UtcOffset {
    central_europe().local_offset(Timestamp::from_unix_seconds(seconds))
}

fn hours(hours: i32) -> UtcOffset {
    UtcOffset::from_seconds(hours * HOUR).unwrap()
}

#[test]
fn local_offset_in_winter_is_standard_time() {
    // 2026-01-15 12:00 UTC
    assert_eq!(offset_at(1_768_478_400), hours(1));
}

#[test]
fn local_offset_in_summer_is_daylight_saving_time() {
    // 2026-07-15 12:00 UTC
    assert_eq!(offset_at(1_784_116_800), hours(2));
}

#[test]
fn local_offset_changes_exactly_at_the_transitions() {
    // 2026-03-29 01:00 UTC, when clocks go from 02:00 to 03:00
    let spring = 1_774_746_000;
    assert_eq!(offset_at(spring - 1), hours(1));
    assert_eq!(offset_at(spring), hours(2));
    // 2026-10-25 01:00 UTC, when clocks go from 03:00 back to 02:00
    let autumn = 1_792_890_000;
    assert_eq!(offset_at(autumn - 1), hours(2));
    assert_eq!(offset_at(autumn), hours(1));
}

#[test]
fn local_offset_in_the_last_second_before_a_transition_ignores_the_fraction() {
    let just_before_spring = Timestamp::new(1_774_746_000 - 1, 999_999_999).unwrap();
    assert_eq!(central_europe().local_offset(just_before_spring), hours(1));
}

#[test]
fn local_offset_before_1970() {
    // 1960-01-15 and 1960-07-15, 12:00 UTC; a POSIX rule applies to every year.
    assert_eq!(offset_at(-314_366_400), hours(1));
    assert_eq!(offset_at(-298_641_600), hours(2));
}

#[test]
fn local_offset_far_outside_the_range_of_jiff_is_still_given() {
    assert_eq!(offset_at(i64::MIN), hours(1));
    assert_eq!(offset_at(i64::MAX), hours(1));
}

#[test]
fn converts_jiff_timestamps_with_nanoseconds() {
    let time = jiff::Timestamp::new(1_792_245_908, 123_456_789).unwrap();
    assert_eq!(
        to_timestamp(time),
        Timestamp::new(1_792_245_908, 123_456_789).unwrap()
    );
}

#[test]
fn converts_jiff_timestamps_before_the_epoch_counting_nanoseconds_forward() {
    let quarter_before = jiff::Timestamp::new(0, -250_000_000).unwrap();
    assert_eq!(
        to_timestamp(quarter_before),
        Timestamp::new(-1, 750_000_000).unwrap()
    );
    let before = jiff::Timestamp::new(-86_401, -1).unwrap();
    assert_eq!(
        to_timestamp(before),
        Timestamp::new(-86_402, 999_999_999).unwrap()
    );
    let whole = jiff::Timestamp::new(-86_401, 0).unwrap();
    assert_eq!(to_timestamp(whole), Timestamp::from_unix_seconds(-86_401));
}

#[test]
fn converts_timestamps_to_jiff_to_the_whole_second() {
    let time = Timestamp::new(-1, 750_000_000).unwrap();
    assert_eq!(to_jiff_timestamp(time).as_second(), -1);
    assert_eq!(
        to_jiff_timestamp(Timestamp::from_unix_seconds(i64::MIN)),
        jiff::Timestamp::MIN
    );
}

#[test]
fn converts_offsets_and_falls_back_to_utc_beyond_a_day() {
    let east = Offset::from_seconds(5 * HOUR + 45 * 60).unwrap();
    assert_eq!(to_utc_offset(east).seconds(), 5 * HOUR + 45 * 60);
    let west = Offset::from_seconds(-(9 * HOUR + 30 * 60)).unwrap();
    assert_eq!(to_utc_offset(west).seconds(), -(9 * HOUR + 30 * 60));
    let beyond = Offset::from_seconds(25 * HOUR).unwrap();
    assert_eq!(to_utc_offset(beyond), UtcOffset::UTC);
}
