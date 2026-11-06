// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Dating the found files of an import run by capture time or modification time.

use std::path::Path;

use domain::{CaptureTime, LocalDateTime, Timestamp};
use metadata::{CaptureTimeError, NoCaptureTime, read_capture_time};

use super::ImportRun;
use crate::plan::AssetFile;
use crate::report::FileError;
use crate::scan::FoundFile;

/// A found file with its times read, ready for the import plan.
pub(super) struct DatedFile {
    pub(super) asset_file: AssetFile,
    pub(super) no_capture_time: Option<NoCaptureTime>,
    /// The capture time as the file records it, before camera time correction: what
    /// the import history looks the file up by (IMPORT-009).
    pub(super) recorded_capture_time: Option<LocalDateTime>,
}

impl ImportRun<'_> {
    /// Reads the capture time and the local modification time of `file`.
    pub(super) fn date(&self, source: &Path, file: &FoundFile) -> Result<DatedFile, FileError> {
        let file_name = file
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(FileError::NameNotUtf8)?;
        let (capture_time, no_capture_time) =
            match read_capture_time(self.source_file_system, &source.join(&file.path)) {
                Ok(capture_time) => (Some(capture_time), None),
                Err(CaptureTimeError::NoCaptureTime(reason)) => (None, Some(reason)),
                Err(CaptureTimeError::Io { error, .. }) => {
                    return Err(FileError::CannotBeRead { error });
                }
            };
        let modification_time = self
            .local_time(file.modification_time)
            .ok_or(FileError::ModificationTimeOutOfRange)?;
        // No camera time correction exists yet (N15), so the capture time of the plan
        // is the one the file records.
        let recorded_capture_time = capture_time.map(|time: CaptureTime| time.date_time());
        Ok(DatedFile {
            recorded_capture_time,
            asset_file: AssetFile {
                file_name: file_name.to_owned(),
                role: file.role,
                capture_time,
                modification_time,
            },
            no_capture_time,
        })
    }

    /// The local time of `instant` in the clock's time zone.
    pub(super) fn local_time(&self, instant: Timestamp) -> Option<LocalDateTime> {
        LocalDateTime::from_timestamp(instant, self.clock.local_offset(instant))
    }
}
