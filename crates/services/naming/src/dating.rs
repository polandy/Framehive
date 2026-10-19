// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Which time `{date:…}` shows for an asset (IMPORT-001, "Capture time" and "Assets").

use crate::LocalDateTime;

/// The times known for one file of an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileTimes {
    /// Whether the file is the asset's RAW file.
    pub is_raw: bool,
    /// The capture time, after camera time correction, if the file records one.
    pub capture_time: Option<LocalDateTime>,
    /// The file modification time.
    pub modification_time: LocalDateTime,
}

/// Where the time of an asset comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateSource {
    /// A file of the asset recorded a capture time.
    CaptureTime,
    /// No file recorded one; the import plan marks the asset so the user can see it.
    ModificationTime,
}

/// The time all files of an asset are rendered with, and where it comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetDate {
    /// The time for `{date:…}`.
    pub time: LocalDateTime,
    /// Where it comes from.
    pub source: DateSource,
}

/// Chooses the time of an asset from the times of its files.
///
/// The RAW file comes first, then the other files in the given order. The first
/// capture time wins. Without any capture time, the modification time of the first
/// file in that order is used. Returns `None` for an asset without files.
///
/// ```
/// use naming::{DateSource, FileTimes, LocalDateTime, asset_date};
///
/// let raw = LocalDateTime::new(2026, 10, 17, 23, 59, 59)?;
/// let jpeg_modified = LocalDateTime::new(2026, 10, 18, 0, 0, 1)?;
/// let files = [
///     FileTimes { is_raw: false, capture_time: None, modification_time: jpeg_modified },
///     FileTimes { is_raw: true, capture_time: Some(raw), modification_time: raw },
/// ];
/// let date = asset_date(&files).unwrap();
/// assert_eq!((date.time, date.source), (raw, DateSource::CaptureTime));
/// # Ok::<(), naming::DateTimeError>(())
/// ```
#[must_use]
pub fn asset_date(files: &[FileTimes]) -> Option<AssetDate> {
    let raw_first = || {
        let raw = files.iter().filter(|file| file.is_raw);
        raw.chain(files.iter().filter(|file| !file.is_raw))
    };
    let captured = raw_first()
        .find_map(|file| file.capture_time)
        .map(|time| AssetDate {
            time,
            source: DateSource::CaptureTime,
        });
    captured.or_else(|| {
        raw_first().next().map(|file| AssetDate {
            time: file.modification_time,
            source: DateSource::ModificationTime,
        })
    })
}
