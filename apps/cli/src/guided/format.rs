// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The numbers the guided import shows: sizes, the transfer rate and the time left
//! (CLI-003, "Plan summary" and "Import and progress").

use std::time::Duration;

/// One gigabyte, decimal.
const GIGABYTE: u64 = 1_000_000_000;
/// One megabyte, decimal.
const MEGABYTE: u64 = 1_000_000;

/// The unit a size is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Megabytes,
    Gigabytes,
}

impl Unit {
    /// Gigabytes from one gigabyte on, otherwise megabytes.
    fn for_size(bytes: u64) -> Self {
        if bytes >= GIGABYTE {
            Unit::Gigabytes
        } else {
            Unit::Megabytes
        }
    }

    fn bytes(self) -> u64 {
        match self {
            Unit::Megabytes => MEGABYTE,
            Unit::Gigabytes => GIGABYTE,
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Unit::Megabytes => "MB",
            Unit::Gigabytes => "GB",
        }
    }

    /// `bytes` in this unit with one decimal, rounded half up, without the symbol.
    fn number(self, bytes: u64) -> String {
        let tenth = u128::from(self.bytes() / 10);
        let tenths = (u128::from(bytes) + tenth / 2) / tenth;
        format!("{}.{}", tenths / 10, tenths % 10)
    }
}

/// A size with one decimal: `12.8 GB` from one gigabyte on, otherwise `30.0 MB`.
pub(crate) fn size_text(bytes: u64) -> String {
    let unit = Unit::for_size(bytes);
    format!("{} {}", unit.number(bytes), unit.symbol())
}

/// Two sizes in the unit of `total`: `4.1/12.8 GB`.
fn sizes_text(done: u64, total: u64) -> String {
    let unit = Unit::for_size(total);
    format!(
        "{}/{} {}",
        unit.number(done),
        unit.number(total),
        unit.symbol()
    )
}

/// How far an import is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Progress {
    /// The files that are done.
    pub(crate) done_files: usize,
    /// All files of the import.
    pub(crate) total_files: usize,
    /// The bytes of the files that are done.
    pub(crate) done_bytes: u64,
    /// The bytes of all files.
    pub(crate) total_bytes: u64,
    /// The time since the import started, by the clock.
    pub(crate) elapsed: Duration,
}

/// The progress line: `  312/1024 files  4.1/12.8 GB  85 MB/s  1:42 left`, or with
/// `-- MB/s  --:-- left` until the rate is known.
pub(crate) fn progress_line(progress: &Progress) -> String {
    let rate = rate_and_time_left(progress).unwrap_or_else(|| "-- MB/s  --:-- left".to_owned());
    format!(
        "  {}/{} files  {}  {rate}",
        progress.done_files,
        progress.total_files,
        sizes_text(progress.done_bytes, progress.total_bytes)
    )
}

/// The rate in whole MB/s and the time left at that rate, or `None` while no byte is
/// done or no time has passed.
fn rate_and_time_left(progress: &Progress) -> Option<String> {
    let elapsed_nanos = progress.elapsed.as_nanos();
    let done = u128::from(progress.done_bytes);
    if done == 0 || elapsed_nanos == 0 {
        return None;
    }
    let bytes_per_second = done * 1_000_000_000 / elapsed_nanos;
    let megabytes = u128::from(MEGABYTE);
    let rate = (bytes_per_second + megabytes / 2) / megabytes;
    let remaining = u128::from(progress.total_bytes.saturating_sub(progress.done_bytes));
    // remaining / (done / elapsed), rounded up to whole seconds.
    let seconds_left = (remaining * elapsed_nanos).div_ceil(done * 1_000_000_000);
    Some(format!("{rate} MB/s  {} left", duration_text(seconds_left)))
}

/// `m:ss`, or `h:mm:ss` from one hour on.
fn duration_text(seconds: u128) -> String {
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// spec: CLI-003.1
    #[test]
    fn sizes_are_decimal_with_one_digit() {
        assert_eq!(size_text(3_000_000_000), "3.0 GB");
        assert_eq!(size_text(12_849_999_999), "12.8 GB");
        assert_eq!(size_text(999_000_000), "999.0 MB");
        assert_eq!(size_text(30_000_000), "30.0 MB");
        assert_eq!(size_text(0), "0.0 MB");
    }

    /// spec: CLI-003.5
    #[test]
    fn the_progress_line_shows_files_sizes_rate_and_time_left() {
        let progress = Progress {
            done_files: 312,
            total_files: 1024,
            done_bytes: 4_100_000_000,
            total_bytes: 12_800_000_000,
            elapsed: Duration::from_secs(48),
        };

        assert_eq!(
            progress_line(&progress),
            "  312/1024 files  4.1/12.8 GB  85 MB/s  1:42 left"
        );
    }

    /// spec: CLI-003.5
    #[test]
    fn the_rate_is_unknown_until_bytes_and_time_have_passed() {
        let progress = Progress {
            total_files: 3,
            total_bytes: 30_000_000,
            ..Progress::default()
        };

        assert_eq!(
            progress_line(&progress),
            "  0/3 files  0.0/30.0 MB  -- MB/s  --:-- left"
        );
    }

    #[test]
    fn an_hour_or_more_left_shows_hours() {
        assert_eq!(duration_text(3_600), "1:00:00");
        assert_eq!(duration_text(3_599), "59:59");
        assert_eq!(duration_text(0), "0:00");
    }
}
