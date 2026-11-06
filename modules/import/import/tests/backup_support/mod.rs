// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A source, an import run with backup destinations and the expected outcomes shared
//! by the backup tests (IMPORT-007).

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers may panic on bad fixtures"
)]
#![allow(dead_code, reason = "each test binary uses only some of the helpers")]

use std::path::{Path, PathBuf};

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{FnvHasher, MemoryFileSystem, MemoryFreeSpace};
use history::testing::MemoryImportHistory;
use import::{
    BackupError, BackupOutcome, ConflictPolicy, FileOutcome, FileReport, ImportRun, ImportSettings,
    ImportSummary, NoProgress, PlannedRun, RunError, SessionSequence, scan_source,
};
use naming::testing::MemorySequenceCounters;
use naming::{CompatibilityMode, NameTemplate};

pub(crate) const SOURCE: &str = "/media/card";
pub(crate) const DESTINATION: &str = "/library";
pub(crate) const BACKUP: &str = "/backup-1";
pub(crate) const SECOND_BACKUP: &str = "/backup-2";
pub(crate) const FLAT: &str = "{name}.{ext}";
pub(crate) const BY_YEAR: &str = "{date:%Y}/{name}.{ext}";

/// 2026-10-17 14:05:09 UTC.
pub(crate) const MODIFIED: i64 = 1_792_245_909;
/// 2026-10-31 09:00:00 UTC.
pub(crate) const NOW: i64 = 1_793_437_200;

pub(crate) fn modified() -> Timestamp {
    Timestamp::from_unix_seconds(MODIFIED)
}

/// A file system with the source files `(name, content)` at the top of the source.
pub(crate) fn source_with(files: &[(&str, &[u8])]) -> MemoryFileSystem {
    let file_system = MemoryFileSystem::new();
    file_system.add_folder(Path::new(SOURCE));
    file_system.add_folder(Path::new(DESTINATION));
    for (name, content) in files {
        file_system.add_file(&Path::new(SOURCE).join(name), content, modified());
    }
    file_system
}

pub(crate) fn at(root: &str, relative: &str) -> PathBuf {
    Path::new(root).join(relative)
}

pub(crate) fn part(root: &str, relative: &str) -> PathBuf {
    at(root, &format!("{relative}.part"))
}

pub(crate) struct Setup<'a> {
    template: &'a str,
    policy: ConflictPolicy,
    backups: Vec<PathBuf>,
    free_space: MemoryFreeSpace,
    dry_run: bool,
}

impl<'a> Setup<'a> {
    pub(crate) fn new(template: &'a str, backups: &[&str]) -> Self {
        Setup {
            template,
            policy: ConflictPolicy::AddSuffix,
            backups: backups.iter().map(PathBuf::from).collect(),
            free_space: MemoryFreeSpace::unlimited(),
            dry_run: false,
        }
    }

    /// The free space of the destinations (IMPORT-012); unlimited by default.
    pub(crate) fn with_free_space(mut self, free_space: MemoryFreeSpace) -> Self {
        self.free_space = free_space;
        self
    }

    pub(crate) fn dry_running(mut self) -> Self {
        self.dry_run = true;
        self
    }

    pub(crate) fn with_policy(mut self, policy: ConflictPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub(crate) fn on(&self, file_system: &MemoryFileSystem) -> ImportSummary {
        self.try_on(file_system).unwrap()
    }

    pub(crate) fn try_on(&self, file_system: &MemoryFileSystem) -> Result<ImportSummary, RunError> {
        self.with_run(file_system, |run, settings| {
            run.run(settings, &mut NoProgress)
        })
    }

    /// Plans the import, calls `before_start`, and starts it, so a test can change
    /// the files between planning and starting.
    pub(crate) fn try_start_on(
        &self,
        file_system: &MemoryFileSystem,
        before_start: impl FnOnce(),
    ) -> Result<PlannedRun, RunError> {
        self.with_run(file_system, |run, settings| {
            let scan = scan_source(file_system, settings.source, settings.destination).unwrap();
            let plan = run.plan(settings, &scan)?;
            before_start();
            run.start(settings, plan)
        })
    }

    /// Calls `act` with an import run over `file_system` and the settings of the setup.
    fn with_run<T>(
        &self,
        file_system: &MemoryFileSystem,
        act: impl FnOnce(&ImportRun<'_>, &ImportSettings<'_>) -> T,
    ) -> T {
        let clock = ManualClock::new(
            Timestamp::from_unix_seconds(NOW),
            UtcOffset::from_seconds(0).unwrap(),
        );
        let template = NameTemplate::parse(self.template).unwrap();
        let settings = ImportSettings {
            source: Path::new(SOURCE),
            destination: Path::new(DESTINATION),
            backup_destinations: &self.backups,
            template: &template,
            job_code: None,
            mode: CompatibilityMode::On,
            policy: self.policy,
            dry_run: self.dry_run,
        };
        let (counters, session) = (MemorySequenceCounters::new(), SessionSequence::new());
        let history = MemoryImportHistory::new();
        let run = ImportRun::new(
            file_system,
            &FnvHasher,
            &clock,
            &counters,
            &session,
            &history,
            &self.free_space,
        );
        act(&run, &settings)
    }
}

pub(crate) fn only_file(summary: &ImportSummary) -> &FileReport {
    let [file] = &summary.files[..] else {
        panic!("one file expected, got {:?}", summary.files);
    };
    file
}

pub(crate) fn only_backup(report: &FileReport) -> &BackupOutcome {
    let [backup] = &report.backups[..] else {
        panic!("one backup expected, got {:?}", report.backups);
    };
    assert_eq!(backup.destination, Path::new(BACKUP));
    &backup.outcome
}

pub(crate) fn copied_target(report: &FileReport) -> &str {
    match &report.outcome {
        FileOutcome::Copied { target, .. } => target,
        other => panic!("copied expected, got {other:?}"),
    }
}

pub(crate) fn backup_copy_error(outcome: &BackupOutcome) -> &files::CopyError {
    match outcome {
        BackupOutcome::Failed(BackupError::Copy(error)) => error,
        other => panic!("a failed copy expected, got {other:?}"),
    }
}

pub(crate) fn paths_below(file_system: &MemoryFileSystem, root: &str) -> Vec<PathBuf> {
    file_system
        .paths()
        .into_iter()
        .filter(|path| path.starts_with(root) && path != Path::new(root))
        .collect()
}
