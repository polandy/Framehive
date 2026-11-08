// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import history in an import run: recognizing files imported before and backed
//! up before when the plan is made, and recording each verified copy (IMPORT-009).

use std::path::{Path, PathBuf};

use domain::{Checksum, LocalDateTime};
use files::checksum_file;
use history::{
    CopyKey, CopyRecord, FoundCopy, HistoryError, RecordedTime, RunId, RunStart, SidecarRecord,
};

use super::dating::DatedFile;
use super::{ImportRun, ImportSettings};
use crate::backup::BackupOutcome;
use crate::report::{FileError, FileOutcome, FileReport, RunError};
use crate::scan::FoundFile;

/// What the import history records a file by, and what it found for it.
#[derive(Debug)]
pub(super) struct Recognition {
    /// The file name on the source.
    file_name: String,
    /// The size in bytes.
    size: u64,
    /// The capture time as the file records it, before camera time correction.
    capture_time: Option<LocalDateTime>,
    /// The checksum of the source, when the history held a candidate and it was read.
    checksum: Option<Checksum>,
    /// The backup destinations the file is backed up before at.
    backed_up_before: Vec<PathBuf>,
}

impl Recognition {
    /// Whether the file is backed up before at the backup destination `destination`.
    pub(super) fn is_backed_up_before(&self, destination: &Path) -> bool {
        self.backed_up_before
            .iter()
            .any(|before| before == destination)
    }

    /// The checksum of the source, if the lookup read it.
    pub(super) fn checksum(&self) -> Option<&Checksum> {
        self.checksum.as_ref()
    }
}

/// A run that the import history recorded the start of.
#[derive(Debug, Clone, Copy)]
pub(super) struct RecordedRun {
    pub(super) id: RunId,
    pub(super) started_at: RecordedTime,
}

/// A dated file looked up in the import history.
pub(super) struct Recognized {
    pub(super) dated: DatedFile,
    pub(super) recognition: Recognition,
    /// The target of the most recent copy at the import destination with the same
    /// content, if the file is imported before.
    pub(super) imported_before: Option<String>,
}

impl ImportRun<'_> {
    /// Now, as the import history records times; `None` outside the years 1 to 9999.
    pub(super) fn recorded_now(&self) -> Option<RecordedTime> {
        let now = self.clock.now();
        RecordedTime::at(now, self.clock.local_offset(now))
    }

    /// Records the start of the run in the import history.
    pub(super) fn start_recorded_run(
        &self,
        settings: &ImportSettings<'_>,
    ) -> Result<RecordedRun, RunError> {
        let started_at = self.recorded_now().ok_or(RunError::ImportTimeOutOfRange)?;
        let id = self.history.start_run(&RunStart {
            source: self.source_name.unwrap_or(settings.source).to_path_buf(),
            import_destination: settings.destination.to_path_buf(),
            video_destination: settings.separate_video_destination().map(Path::to_path_buf),
            started_at,
        })?;
        Ok(RecordedRun { id, started_at })
    }

    /// Records the end of `run` in the import history.
    pub(super) fn finish_recorded_run(&self, run: RecordedRun) -> Result<(), HistoryError> {
        // A clock that left the years 1 to 9999 during the run cannot be recorded; the
        // start of the run is the closest time that can.
        let finished_at = self.recorded_now().unwrap_or(run.started_at);
        self.history.finish_run(run.id, finished_at)
    }

    /// Looks `file` up at its own import destination and at each of its own backup
    /// destinations, the video ones for a video (IMPORT-013). The source is read for its
    /// checksum only when a lookup finds a copy with the same name, size and capture
    /// time (IMPORT-009.5).
    ///
    /// The outer error stops the plan: the history cannot be read. The inner one marks
    /// the file: its content cannot be read.
    pub(super) fn recognize(
        &self,
        settings: &ImportSettings<'_>,
        file: &FoundFile,
        dated: DatedFile,
    ) -> Result<Result<Recognized, FileError>, HistoryError> {
        let at_import_destination =
            self.find_copies(settings.destination_of(file.role), file, &dated)?;
        let backup_destinations = settings.backup_destinations_of(file.role);
        let mut at_backups = Vec::with_capacity(backup_destinations.len());
        for destination in backup_destinations {
            at_backups.push((destination, self.find_copies(destination, file, &dated)?));
        }
        let mut recognized = Recognized {
            recognition: Recognition {
                file_name: dated.asset_file.file_name.clone(),
                size: file.size,
                capture_time: dated.recorded_capture_time,
                checksum: None,
                backed_up_before: Vec::new(),
            },
            dated,
            imported_before: None,
        };
        let candidates = !at_import_destination.is_empty()
            || at_backups.iter().any(|(_, copies)| !copies.is_empty());
        if !candidates {
            return Ok(Ok(recognized));
        }
        let source = settings.source.join(&file.path);
        let checksum = match checksum_file(self.source_file_system, self.hasher, &source) {
            Ok(checksum) => checksum,
            Err(error) => return Ok(Err(FileError::CannotBeRead { error })),
        };
        // The copies come oldest first, so the last equal one is the most recent.
        recognized.imported_before = at_import_destination
            .into_iter()
            .rev()
            .find(|copy| copy.checksum == checksum)
            .map(|copy| copy.target);
        recognized.recognition.backed_up_before = at_backups
            .into_iter()
            .filter(|(_, copies)| copies.iter().any(|copy| copy.checksum == checksum))
            .map(|(destination, _)| destination.clone())
            .collect();
        recognized.recognition.checksum = Some(checksum);
        Ok(Ok(recognized))
    }

    /// The recorded copies at `destination` with the name, size and capture time of
    /// `file`.
    fn find_copies(
        &self,
        destination: &Path,
        file: &FoundFile,
        dated: &DatedFile,
    ) -> Result<Vec<FoundCopy>, HistoryError> {
        self.history.find_copies(&CopyKey {
            destination,
            original_file_name: &dated.asset_file.file_name,
            size: file.size,
            capture_time: dated.recorded_capture_time,
        })
    }

    /// Records each copy of `report` that was verified in this run: at `destination`,
    /// the file's import destination, when the file was copied or already imported,
    /// and at each backup destination where it was backed up or already backed up.
    /// `checksum` is the source's, when the run read it; `sidecar` is the one written
    /// next to each of these copies (IMPORT-019.15).
    pub(super) fn record_copies(
        &self,
        destination: &Path,
        run: RecordedRun,
        recognition: &Recognition,
        report: &FileReport,
        checksum: Option<&Checksum>,
        sidecar: Option<&SidecarRecord>,
    ) -> Result<(), HistoryError> {
        let (target, at_import_destination, checksum) = match &report.outcome {
            FileOutcome::Copied { target, checksum } => (target, true, Some(checksum)),
            FileOutcome::AlreadyImported { target } => (target, true, checksum),
            FileOutcome::ImportedBefore { target } => (target, false, checksum),
            _ => return Ok(()),
        };
        let checksum = checksum.or(recognition.checksum());
        // Every verified copy and every comparison reads the source, so a checksum is
        // missing only when nothing was verified.
        let Some(checksum) = checksum else {
            return Ok(());
        };
        let backed_up = report.backups.iter().filter(|backup| {
            matches!(
                backup.outcome,
                BackupOutcome::BackedUp | BackupOutcome::AlreadyBackedUp
            )
        });
        let destinations = at_import_destination
            .then_some(destination)
            .into_iter()
            .chain(backed_up.map(|backup| backup.destination.as_path()));
        for destination in destinations {
            self.history.record_copy(&CopyRecord {
                run: run.id,
                destination: destination.to_path_buf(),
                source_path: report.path.clone(),
                original_file_name: recognition.file_name.clone(),
                size: recognition.size,
                capture_time: recognition.capture_time,
                checksum: checksum.clone(),
                target: target.clone(),
                // A clock that left the years 1 to 9999 during the run cannot be
                // recorded; the start of the run is the closest time that can.
                recorded_at: self.recorded_now().unwrap_or(run.started_at),
                sidecar: sidecar.cloned(),
            })?;
        }
        Ok(())
    }
}
