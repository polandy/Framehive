// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A run and its log for the unit tests of the import log formats.

#![allow(clippy::unwrap_used, reason = "fixed, valid fixture values")]

use std::path::PathBuf;

use domain::{Checksum, LocalDateTime, Timestamp, UtcOffset};
use history::{CopyRecord, LoggedRun, RecordedTime, RunId, RunLog, RunStart};

pub(super) fn now() -> RecordedTime {
    RecordedTime::at(
        Timestamp::from_unix_seconds(1_793_437_200),
        UtcOffset::from_seconds(7_200).unwrap(),
    )
    .unwrap()
}

/// Run 3 from `/media/card` into `/library`, with two copies.
pub(super) fn run(finished_at: Option<RecordedTime>) -> LoggedRun {
    LoggedRun {
        id: RunId::new(3),
        start: RunStart {
            source: PathBuf::from("/media/card"),
            import_destination: PathBuf::from("/library"),
            started_at: now(),
        },
        finished_at,
        copy_count: 2,
    }
}

/// A copy of `DCIM/a,"b".jpg`, captured 2026-10-17 14:05:09, at `destination`.
pub(super) fn copy(destination: &str) -> CopyRecord {
    CopyRecord {
        run: RunId::new(3),
        destination: PathBuf::from(destination),
        source_path: PathBuf::from("DCIM/a,\"b\".jpg"),
        original_file_name: "a,\"b\".jpg".to_owned(),
        size: 10,
        capture_time: Some(LocalDateTime::new(2026, 10, 17, 14, 5, 9).unwrap()),
        checksum: Checksum::new(Checksum::BLAKE3, vec![1, 1]).unwrap(),
        target: "2026/a,\"b\".jpg".to_owned(),
        recorded_at: now(),
    }
}

/// The finished run 3 with its copy at `/library` and at `/backup`.
pub(super) fn log() -> RunLog {
    RunLog {
        run: run(Some(now())),
        copies: vec![copy("/library"), copy("/backup")],
    }
}
