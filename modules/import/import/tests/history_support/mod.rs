// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A machine with an import history, a source and the imports of the import history
//! tests (IMPORT-009), against `MemoryFileSystem` and `MemoryImportHistory`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers may panic on bad fixtures"
)]
#![allow(dead_code, reason = "each test binary uses only some of the helpers")]

use std::cell::Cell;
use std::path::{Path, PathBuf};

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{FnvHasher, MemoryFileSystem};
use files::{HashState, Hasher};
use history::testing::MemoryImportHistory;
use import::{
    ConflictPolicy, FileOutcome, FileReport, ImportRun, ImportSettings, ImportSummary, NoProgress,
    ProgressSink, RunError, SessionSequence, SourceScan,
};
use metadata::testing::{ByteOrder, DATE_TIME_ORIGINAL, Tiff, jpeg};
use naming::testing::MemorySequenceCounters;
use naming::{CompatibilityMode, NameTemplate};

pub(crate) const SOURCE: &str = "/media/card";
pub(crate) const DESTINATION: &str = "/library";
pub(crate) const OTHER_DESTINATION: &str = "/other-library";
pub(crate) const BACKUP: &str = "/backup";
pub(crate) const BY_DATE: &str = "{date:%Y-%m-%d}/{name}.{ext}";
pub(crate) const FLAT: &str = "{name}.{ext}";
pub(crate) const STORED: &str = "{seq_stored:4}.{ext}";
/// 2026-10-17 14:05:09 UTC.
pub(crate) const MODIFIED: i64 = 1_792_245_909;
/// 2026-10-31 09:00:00 UTC.
pub(crate) const NOW: i64 = 1_793_437_200;

/// A JPEG taken at 2026-10-17 14:05:09, made distinct by `variant` after its end.
pub(crate) fn photo(variant: u8) -> Vec<u8> {
    let exif = Tiff::new(ByteOrder::LittleEndian)
        .with_exif_text(DATE_TIME_ORIGINAL, "2026:10:17 14:05:09")
        .to_bytes();
    let mut content = jpeg(&exif);
    content.push(variant);
    content
}

/// A file system with the folders of the tests and the source files `(name, content)`.
pub(crate) fn source_with(files: &[(&str, &[u8])]) -> MemoryFileSystem {
    let file_system = MemoryFileSystem::new();
    for folder in [SOURCE, DESTINATION, OTHER_DESTINATION, BACKUP] {
        file_system.add_folder(Path::new(folder));
    }
    for (name, content) in files {
        add(&file_system, SOURCE, name, content);
    }
    file_system
}

pub(crate) fn add(file_system: &MemoryFileSystem, root: &str, relative: &str, content: &[u8]) {
    let modified = Timestamp::from_unix_seconds(MODIFIED);
    file_system.add_file(&at(root, relative), content, modified);
}

pub(crate) fn at(root: &str, relative: &str) -> PathBuf {
    Path::new(root).join(relative)
}

pub(crate) fn paths_below(file_system: &MemoryFileSystem, root: &str) -> Vec<PathBuf> {
    file_system
        .paths()
        .into_iter()
        .filter(|path| path.starts_with(root) && path != Path::new(root))
        .collect()
}

/// [`FnvHasher`] that counts the hashes it starts.
#[derive(Default)]
pub(crate) struct CountingHasher {
    pub(crate) started: Cell<usize>,
}

impl Hasher for CountingHasher {
    fn start(&self) -> Box<dyn HashState> {
        self.started.set(self.started.get() + 1);
        FnvHasher.start()
    }
}

/// How one import is set up.
pub(crate) struct Import {
    template: NameTemplate,
    destination: &'static str,
    backups: Vec<PathBuf>,
    policy: ConflictPolicy,
    dry_run: bool,
}

impl Import {
    pub(crate) fn with(template: &str) -> Self {
        Import {
            template: NameTemplate::parse(template).unwrap(),
            destination: DESTINATION,
            backups: Vec::new(),
            policy: ConflictPolicy::AddSuffix,
            dry_run: false,
        }
    }

    pub(crate) fn into(mut self, destination: &'static str) -> Self {
        self.destination = destination;
        self
    }

    pub(crate) fn backed_up(mut self) -> Self {
        self.backups = vec![PathBuf::from(BACKUP)];
        self
    }

    pub(crate) fn with_policy(mut self, policy: ConflictPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub(crate) fn dry_running(mut self) -> Self {
        self.dry_run = true;
        self
    }

    pub(crate) fn settings(&self) -> ImportSettings<'_> {
        ImportSettings {
            source: Path::new(SOURCE),
            destination: Path::new(self.destination),
            backup_destinations: &self.backups,
            template: &self.template,
            job_code: None,
            mode: CompatibilityMode::On,
            policy: self.policy,
            dry_run: self.dry_run,
        }
    }
}

/// What the imports of one test share: the history and the persisted counters, as
/// on one machine. Each import is a new process with its own session counter.
pub(crate) struct Machine {
    pub(crate) history: MemoryImportHistory,
    counters: MemorySequenceCounters,
    pub(crate) hasher: CountingHasher,
    clock: ManualClock,
}

impl Machine {
    pub(crate) fn new() -> Self {
        Machine {
            history: MemoryImportHistory::new(),
            counters: MemorySequenceCounters::new(),
            hasher: CountingHasher::default(),
            clock: ManualClock::new(
                Timestamp::from_unix_seconds(NOW),
                UtcOffset::from_seconds(0).unwrap(),
            ),
        }
    }

    pub(crate) fn import(&self, file_system: &MemoryFileSystem, import: &Import) -> ImportSummary {
        self.try_import(file_system, import, &mut NoProgress)
            .unwrap()
    }

    pub(crate) fn try_import(
        &self,
        file_system: &MemoryFileSystem,
        import: &Import,
        progress: &mut dyn ProgressSink,
    ) -> Result<ImportSummary, RunError> {
        let session = SessionSequence::new();
        self.run(file_system, &session)
            .run(&import.settings(), progress)
    }

    pub(crate) fn run<'a>(
        &'a self,
        file_system: &'a MemoryFileSystem,
        session: &'a SessionSequence,
    ) -> ImportRun<'a> {
        ImportRun::new(
            file_system,
            &self.hasher,
            &self.clock,
            &self.counters,
            session,
            &self.history,
        )
    }

    pub(crate) fn stored_last(&self) -> u32 {
        self.counters.stored().stored_last
    }
}

/// A sink that lets the run go on until `files_before_stop` files are done.
pub(crate) struct StopAfter {
    files_before_stop: usize,
    finished: usize,
}

impl ProgressSink for StopAfter {
    fn scan_finished(&mut self, _scan: &SourceScan) {}
    fn file_started(&mut self, _path: &Path) {}
    fn file_finished(&mut self, _report: &FileReport) {
        self.finished += 1;
    }
    fn should_continue(&mut self) -> bool {
        self.finished < self.files_before_stop
    }
}

pub(crate) fn stop_after(files_before_stop: usize) -> StopAfter {
    StopAfter {
        files_before_stop,
        finished: 0,
    }
}

pub(crate) fn report<'a>(summary: &'a ImportSummary, path: &str) -> &'a FileReport {
    summary
        .files
        .iter()
        .find(|file| file.path == Path::new(path))
        .unwrap_or_else(|| panic!("no report of {path} in {:?}", summary.files))
}

pub(crate) fn imported_before(report: &FileReport) -> &str {
    match &report.outcome {
        FileOutcome::ImportedBefore { target } => target,
        other => panic!("imported before expected, got {other:?}"),
    }
}

pub(crate) fn copied(report: &FileReport) -> &str {
    match &report.outcome {
        FileOutcome::Copied { target, .. } => target,
        other => panic!("copied expected, got {other:?}"),
    }
}

/// The destinations the history holds a copy of `source_path` at.
pub(crate) fn recorded_at(history: &MemoryImportHistory, source_path: &str) -> Vec<PathBuf> {
    history
        .copies()
        .into_iter()
        .filter(|copy| copy.source_path == Path::new(source_path))
        .map(|copy| copy.destination)
        .collect()
}

/// `DSCF0001.JPG` on a card, and the machine that imported it with `template`.
pub(crate) fn imported_once(template: &str) -> (Machine, MemoryFileSystem) {
    let machine = Machine::new();
    let file_system = source_with(&[("DSCF0001.JPG", &photo(1))]);
    machine.import(&file_system, &Import::with(template));
    (machine, file_system)
}
