// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The ports that the import jobs of one application share: one file system, one set
//! of counters, one session and one register of reserved target names (IMPORT-015,
//! ADR 0036).

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{FnvHasher, MemoryFileSystem, MemoryFreeSpace};
use history::testing::MemoryImportHistory;
use import::{
    ConflictPolicy, ImportJob, ImportJobReports, ImportJobSettings, ImportPorts, ImportSettings,
    ReservedTargetNames, SessionSequence,
};
use jobs::{Job, JobControl, JobEnd, ResourceKey};
use metadata::testing::RecordedXml;
use naming::testing::MemorySequenceCounters;
use naming::{CompatibilityMode, NameTemplate, StoredCounters};

use crate::run_support::{MODIFIED, NOW};

/// What the import jobs of one application share.
pub(crate) struct Shared {
    pub(crate) file_system: MemoryFileSystem,
    pub(crate) counters: Arc<MemorySequenceCounters>,
    session: Arc<SessionSequence>,
    history: Arc<MemoryImportHistory>,
    reserved_names: Arc<ReservedTargetNames>,
    free_space: MemoryFreeSpace,
}

impl Shared {
    /// Jobs on an empty file system whose file systems `free_space` tells.
    pub(crate) fn new(free_space: MemoryFreeSpace) -> Self {
        Shared {
            file_system: MemoryFileSystem::new(),
            counters: Arc::new(MemorySequenceCounters::new()),
            session: Arc::new(SessionSequence::new()),
            history: Arc::new(MemoryImportHistory::new()),
            reserved_names: Arc::new(ReservedTargetNames::new()),
            free_space,
        }
    }

    /// Sets the stored counter to `last`.
    pub(crate) fn stored_at(&self, last: u32) {
        self.counters.set(StoredCounters {
            stored_last: last,
            ..StoredCounters::default()
        });
    }

    /// The last number of the stored counter.
    pub(crate) fn stored_last(&self) -> u32 {
        self.counters.stored().stored_last
    }

    /// Adds `names` to the folder `source`, each with its path as content.
    pub(crate) fn source(&self, source: &str, names: &[&str]) {
        self.file_system.add_folder(Path::new(source));
        for name in names {
            let path = Path::new(source).join(name);
            let content = path.to_string_lossy().into_owned();
            let modified = Timestamp::from_unix_seconds(MODIFIED);
            self.file_system
                .add_file(&path, content.as_bytes(), modified);
        }
    }

    /// The ports of one job.
    pub(crate) fn ports(&self) -> ImportPorts {
        let clock = ManualClock::new(Timestamp::from_unix_seconds(NOW), UtcOffset::UTC);
        ImportPorts {
            file_system: Arc::new(self.file_system.clone()),
            hasher: Arc::new(FnvHasher),
            clock: Arc::new(clock),
            counters: Arc::clone(&self.counters) as _,
            session: Arc::clone(&self.session),
            history: Arc::clone(&self.history) as _,
            free_space: Arc::new(self.free_space.clone()),
            reserved_names: Arc::clone(&self.reserved_names),
            xml_syntax: Arc::new(RecordedXml::new()),
        }
    }

    /// A job from `source` to `destination` with `template`, given the plan made on
    /// the caller's thread (IMPORT-015.8).
    pub(crate) fn planned_job(
        &self,
        source: &str,
        destination: &str,
        template: &str,
    ) -> (ImportJob, ImportJobReports) {
        let ports = self.ports();
        let settings = job_settings(source, destination, template);
        let scan =
            import::scan_source(&self.file_system, Path::new(source), Path::new(destination))
                .unwrap();
        let plan = ports
            .import_run()
            .plan(&settings.settings(), &scan)
            .unwrap();
        let (job, reports) = ImportJob::new(ports, settings);
        (job.with_plan(plan), reports)
    }
}

/// The settings of a job from `source` to `destination` with `template`.
pub(crate) fn job_settings(source: &str, destination: &str, template: &str) -> ImportJobSettings {
    let template = NameTemplate::parse(template).unwrap();
    ImportJobSettings::of(&ImportSettings {
        source: Path::new(source),
        destination: Path::new(destination),
        backup_destinations: &[],
        video_destination: None,
        video_backup_destinations: &[],
        template: &template,
        video_template: None,
        job_code: None,
        metadata: import::ImportMetadata::none(),
        mode: CompatibilityMode::On,
        policy: ConflictPolicy::AddSuffix,
        dry_run: false,
    })
}

/// A job that names `resources` and waits until the test opens its gate.
pub(crate) struct GateOn {
    pub(crate) resources: Vec<ResourceKey>,
    pub(crate) gate: Receiver<()>,
}

impl Job for GateOn {
    fn resources(&self) -> Vec<ResourceKey> {
        self.resources.clone()
    }

    fn run(self: Box<Self>, _control: &JobControl) -> JobEnd {
        let _ = self.gate.recv();
        JobEnd::Finished
    }
}
