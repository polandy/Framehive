// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the import history records: runs and verified copies (ADR 0031, section 3).

use std::fmt;
use std::path::{Path, PathBuf};

use domain::{Checksum, LocalDateTime, Timestamp, UtcOffset};

/// A time the history records: the local time of the machine with its UTC offset.
///
/// It is written in RFC 3339, `YYYY-MM-DDTHH:MM:SS±HH:MM` (ADR 0031, section 3).
///
/// ```
/// use domain::{Timestamp, UtcOffset};
/// use history::RecordedTime;
///
/// let offset = UtcOffset::from_seconds(7_200)?;
/// let time = RecordedTime::at(Timestamp::from_unix_seconds(1_793_437_200), offset).unwrap();
/// assert_eq!(time.to_string(), "2026-10-31T11:00:00+02:00");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedTime {
    /// The local date and time.
    pub local: LocalDateTime,
    /// The offset of the local time from UTC.
    pub offset: UtcOffset,
}

impl RecordedTime {
    /// The local time of `instant` at `offset`, or `None` outside the years 1 to
    /// 9999.
    #[must_use]
    pub fn at(instant: Timestamp, offset: UtcOffset) -> Option<Self> {
        let local = LocalDateTime::from_timestamp(instant, offset)?;
        Some(RecordedTime { local, offset })
    }
}

impl fmt::Display for RecordedTime {
    /// Writes RFC 3339. RFC 3339 has no seconds in an offset; they are left out.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.offset.seconds();
        let sign = if seconds < 0 { '-' } else { '+' };
        let minutes = seconds.abs() / 60;
        write!(
            formatter,
            "{}{sign}{:02}:{:02}",
            self.local,
            minutes / 60,
            minutes % 60,
        )
    }
}

/// The id of one recorded run, given by [`ImportHistory::start_run`](crate::ImportHistory::start_run).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunId(i64);

impl RunId {
    /// The run with the number `number`, as the storage counts runs.
    #[must_use]
    pub fn new(number: i64) -> Self {
        RunId(number)
    }

    /// The number of the run.
    #[must_use]
    pub fn number(self) -> i64 {
        self.0
    }
}

/// The start of an import run, as it is recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStart {
    /// The folder the run imports from.
    pub source: PathBuf,
    /// The import destination, as the canonical absolute path the run checks.
    pub import_destination: PathBuf,
    /// The video destination, as the canonical absolute path the run checks, when the
    /// run sends its videos to a folder of their own (IMPORT-013); `None` otherwise.
    pub video_destination: Option<PathBuf>,
    /// When the run started.
    pub started_at: RecordedTime,
}

/// One verified copy of a source file at one destination (ADR 0031, section 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyRecord {
    /// The run that verified the copy.
    pub run: RunId,
    /// The import destination, the video destination or one backup destination, as
    /// the canonical absolute path the run checks.
    pub destination: PathBuf,
    /// The source file, relative to the source.
    pub source_path: PathBuf,
    /// The file name on the source, with its extension.
    pub original_file_name: String,
    /// The size in bytes.
    pub size: u64,
    /// The capture time the file records, before camera time correction, if it has
    /// one.
    pub capture_time: Option<LocalDateTime>,
    /// The checksum of the content.
    pub checksum: Checksum,
    /// The target path relative to `destination`, with `/` between segments.
    pub target: String,
    /// When the copy was recorded.
    pub recorded_at: RecordedTime,
    /// The XMP sidecar the import wrote next to the copy (IMPORT-019), if any.
    pub sidecar: Option<SidecarRecord>,
}

/// A verified XMP sidecar written next to a copy (ADR 0031, addendum of 2026-11-08,
/// IMPORT-019).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarRecord {
    /// The sidecar's path relative to the copy's destination, with `/` between
    /// segments.
    pub target: String,
    /// The checksum of the sidecar's content.
    pub checksum: Checksum,
}

impl CopyRecord {
    /// Whether this copy matches `key`: the same destination, original file name and
    /// size, and, when `key` has a capture time, the same capture time.
    #[must_use]
    pub fn matches(&self, key: &CopyKey<'_>) -> bool {
        self.destination == key.destination
            && self.original_file_name == key.original_file_name
            && self.size == key.size
            && (key.capture_time.is_none() || self.capture_time == key.capture_time)
    }
}

/// What a source file is looked up by: cheap facts that need no read of its content
/// (ADR 0031, section 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyKey<'a> {
    /// The destination to look at.
    pub destination: &'a Path,
    /// The file name on the source.
    pub original_file_name: &'a str,
    /// The size in bytes.
    pub size: u64,
    /// The capture time the file records, before camera time correction. `None`
    /// matches copies with any capture time.
    pub capture_time: Option<LocalDateTime>,
}

/// A recorded copy that matches a [`CopyKey`]: what the run needs to recognize the
/// file by its content and to report it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundCopy {
    /// The target path relative to the destination, with `/` between segments.
    pub target: String,
    /// The checksum of the recorded content.
    pub checksum: Checksum,
}
