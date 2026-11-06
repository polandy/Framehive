// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import log: the recorded runs and the copies of one run, read from the import
//! history (IMPORT-010, ADR 0031, section 7).

use std::path::PathBuf;

use crate::{CopyRecord, RecordedTime, RunId, RunStart};

/// One recorded run as the import log lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggedRun {
    /// The number the history gave the run; it never changes and is never reused.
    pub id: RunId,
    /// The source, the import destination, the video destination and the start of the
    /// run.
    pub start: RunStart,
    /// When the run ended; `None` when it is still running or crashed.
    pub finished_at: Option<RecordedTime>,
    /// The number of copies the run recorded, at every destination.
    pub copy_count: u64,
}

/// The log of one run: the run and every copy it recorded, in the order of
/// IMPORT-010.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLog {
    /// The run.
    pub run: LoggedRun,
    /// The copies: those at the import destination first, then those at the video
    /// destination when the run has one, then those at each backup destination in the
    /// order the run first recorded a copy there; within a destination, in the order
    /// they were recorded.
    pub copies: Vec<CopyRecord>,
}

impl RunLog {
    /// The log of `run` with `recorded`, the run's copies in the order they were
    /// recorded, grouped by destination as IMPORT-010 orders them.
    #[must_use]
    pub fn new(run: LoggedRun, recorded: Vec<CopyRecord>) -> Self {
        let order = destination_order(&run.start, &recorded);
        let mut copies = recorded;
        // A stable sort keeps the recorded order within a destination.
        copies.sort_by_key(|copy| {
            order
                .iter()
                .position(|destination| *destination == copy.destination)
        });
        RunLog { run, copies }
    }
}

/// The import destination, the video destination when the run has one, then each
/// other destination in the order of its first copy in `recorded`.
fn destination_order(start: &RunStart, recorded: &[CopyRecord]) -> Vec<PathBuf> {
    let mut order = vec![start.import_destination.clone()];
    order.extend(start.video_destination.clone());
    for copy in recorded {
        if !order.contains(&copy.destination) {
            order.push(copy.destination.clone());
        }
    }
    order
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

    use super::*;
    use domain::{Checksum, Timestamp, UtcOffset};

    fn now() -> RecordedTime {
        RecordedTime::at(Timestamp::UNIX_EPOCH, UtcOffset::from_seconds(0).unwrap()).unwrap()
    }

    fn run() -> LoggedRun {
        LoggedRun {
            id: RunId::new(1),
            start: RunStart {
                source: PathBuf::from("/media/card"),
                import_destination: PathBuf::from("/library"),
                video_destination: None,
                started_at: now(),
            },
            finished_at: None,
            copy_count: 4,
        }
    }

    fn copy(destination: &str, name: &str) -> CopyRecord {
        CopyRecord {
            run: RunId::new(1),
            destination: PathBuf::from(destination),
            source_path: PathBuf::from(name),
            original_file_name: name.to_owned(),
            size: 1,
            capture_time: None,
            checksum: Checksum::new(Checksum::BLAKE3, vec![1]).unwrap(),
            target: name.to_owned(),
            recorded_at: now(),
        }
    }

    /// spec: IMPORT-010.3
    #[test]
    fn copies_are_grouped_import_destination_first_then_backups_by_first_copy() {
        let recorded = vec![
            copy("/second", "a"),
            copy("/first", "a"),
            copy("/library", "a"),
            copy("/first", "b"),
            copy("/library", "b"),
            copy("/second", "b"),
        ];

        let log = RunLog::new(run(), recorded);

        let order: Vec<(String, String)> = log
            .copies
            .iter()
            .map(|copy| {
                (
                    copy.destination.display().to_string(),
                    copy.original_file_name.clone(),
                )
            })
            .collect();
        let expected = [
            ("/library", "a"),
            ("/library", "b"),
            ("/second", "a"),
            ("/second", "b"),
            ("/first", "a"),
            ("/first", "b"),
        ]
        .map(|(destination, name)| (destination.to_owned(), name.to_owned()));
        assert_eq!(order, expected);
    }

    /// spec: IMPORT-010.9
    #[test]
    fn copies_at_the_video_destination_come_after_the_import_destination() {
        let mut with_video = run();
        with_video.start.video_destination = Some(PathBuf::from("/videos"));
        let recorded = vec![
            copy("/library", "DSCF0001.RAF"),
            copy("/backup", "DSCF0001.RAF"),
            copy("/videos", "clip.MOV"),
            copy("/backup", "clip.MOV"),
        ];

        let log = RunLog::new(with_video, recorded);

        let order: Vec<(&str, &str)> = log
            .copies
            .iter()
            .map(|copy| {
                (
                    copy.destination.to_str().unwrap(),
                    copy.original_file_name.as_str(),
                )
            })
            .collect();
        assert_eq!(
            order,
            [
                ("/library", "DSCF0001.RAF"),
                ("/videos", "clip.MOV"),
                ("/backup", "DSCF0001.RAF"),
                ("/backup", "clip.MOV"),
            ]
        );
    }
}
