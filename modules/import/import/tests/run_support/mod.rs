// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The sources, the import run and the outcomes shared by the import run tests (ADR
//! 0020 §1, IMPORT-004.11).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]
#![allow(dead_code, reason = "each test binary uses only some of the helpers")]

use std::path::Path;

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{FnvHasher, MemoryFileSystem, MemoryFreeSpace};
use history::testing::MemoryImportHistory;
use import::{
    ConflictPolicy, FileError, FileOutcome, FileReport, ImportRun, ImportSettings, ImportSummary,
    NoProgress, ProgressSink, RunError, SessionSequence,
};
use metadata::testing::{ByteOrder, DATE_TIME_ORIGINAL, Tiff, jpeg};
use naming::testing::MemorySequenceCounters;
use naming::{CompatibilityMode, NameTemplate};

pub(crate) const SOURCE: &str = "/media/card";
pub(crate) const DESTINATION: &str = "/library";
pub(crate) const TEMPLATE: &str = "{date:%Y-%m-%d}/{name}.{ext}";

/// 2026-10-17 14:05:09 UTC.
pub(crate) const MODIFIED: i64 = 1_792_245_909;
/// 2026-10-17 23:30:00 UTC: 2026-10-18 in UTC+2.
pub(crate) const MODIFIED_LATE: i64 = 1_792_279_800;
/// 2026-10-31 09:00:00 UTC.
pub(crate) const NOW: i64 = 1_793_437_200;
pub(crate) const TWO_HOURS: i32 = 7_200;

pub(crate) fn source_with(files: &[&str]) -> MemoryFileSystem {
    let file_system = MemoryFileSystem::new();
    file_system.add_folder(Path::new(SOURCE));
    for file in files {
        add(&file_system, file, file.as_bytes(), MODIFIED);
    }
    file_system
}

pub(crate) fn add(file_system: &MemoryFileSystem, relative: &str, content: &[u8], modified: i64) {
    let modified = Timestamp::from_unix_seconds(modified);
    file_system.add_file(&Path::new(SOURCE).join(relative), content, modified);
}

pub(crate) fn photo_taken_at(capture_time: &str) -> Vec<u8> {
    let exif = Tiff::new(ByteOrder::LittleEndian)
        .with_exif_text(DATE_TIME_ORIGINAL, capture_time)
        .to_bytes();
    jpeg(&exif)
}

pub(crate) struct Setup {
    pub(crate) template: &'static str,
    pub(crate) policy: ConflictPolicy,
    pub(crate) dry_run: bool,
    pub(crate) offset: i32,
}

impl Default for Setup {
    fn default() -> Self {
        Setup {
            template: TEMPLATE,
            policy: ConflictPolicy::AddSuffix,
            dry_run: false,
            offset: 0,
        }
    }
}

impl Setup {
    pub(crate) fn on(
        &self,
        file_system: &MemoryFileSystem,
        progress: &mut dyn ProgressSink,
    ) -> ImportSummary {
        self.try_on(file_system, Path::new(DESTINATION), progress)
            .unwrap()
    }

    pub(crate) fn try_on(
        &self,
        file_system: &MemoryFileSystem,
        destination: &Path,
        progress: &mut dyn ProgressSink,
    ) -> Result<ImportSummary, RunError> {
        let offset = UtcOffset::from_seconds(self.offset).unwrap();
        let clock = ManualClock::new(Timestamp::from_unix_seconds(NOW), offset);
        let template = NameTemplate::parse(self.template).unwrap();
        let settings = ImportSettings {
            source: Path::new(SOURCE),
            destination,
            backup_destinations: &[],
            video_destination: None,
            video_backup_destinations: &[],
            template: &template,
            video_template: None,
            job_code: None,
            metadata: import::ImportMetadata::none(),
            mode: CompatibilityMode::On,
            policy: self.policy,
            dry_run: self.dry_run,
        };
        let (counters, session) = (&MemorySequenceCounters::new(), &SessionSequence::new());
        let (history, space) = (&MemoryImportHistory::new(), &MemoryFreeSpace::unlimited());
        ImportRun::new(
            file_system,
            &FnvHasher,
            &clock,
            counters,
            session,
            history,
            space,
        )
        .run(&settings, progress)
    }
}

pub(crate) fn import(file_system: &MemoryFileSystem) -> ImportSummary {
    Setup::default().on(file_system, &mut NoProgress)
}

/// The target of a file that was copied or would be copied.
pub(crate) fn target(report: &FileReport) -> Option<&str> {
    match &report.outcome {
        FileOutcome::Copied { target, .. } | FileOutcome::WouldCopy { target } => Some(target),
        _ => None,
    }
}

pub(crate) fn failure(report: &FileReport) -> Option<&FileError> {
    match &report.outcome {
        FileOutcome::Failed(error) => Some(error),
        _ => None,
    }
}

pub(crate) fn copied_content(file_system: &MemoryFileSystem, target: &str) -> Option<Vec<u8>> {
    file_system.content(&Path::new(DESTINATION).join(target))
}
