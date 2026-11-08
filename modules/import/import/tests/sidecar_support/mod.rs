// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A machine, a card and the imports of the sidecar tests (IMPORT-019), against
//! `MemoryFileSystem`, `MemoryImportHistory` and `RecordedXml`.

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
    ConflictPolicy, FileOutcome, FileReport, ImportMetadata, ImportRun, ImportSettings,
    ImportSummary, JobCode, Keyword, NoProgress, RunError, SessionSequence,
};
use metadata::testing::{ByteOrder, DATE_TIME_ORIGINAL, RecordedXml, Tiff, jpeg, raf};
use metadata::xmp::{SidecarNaming, XmpName, XmpPacket, XmpValue, parse_xmp, serialize_xmp};
use naming::testing::MemorySequenceCounters;
use naming::{CompatibilityMode, NameTemplate};

pub(crate) const SOURCE: &str = "/media/card";
pub(crate) const DESTINATION: &str = "/library";
pub(crate) const BACKUPS: [&str; 2] = ["/backup-1", "/backup-2"];
pub(crate) const FLAT: &str = "{name}.{ext}";
pub(crate) const DATED: &str = "{date:%Y%m%d}-0042.{ext}";
/// 2025-12-31 00:00:00 UTC.
pub(crate) const MODIFIED_2025: i64 = 1_767_139_200;
/// 2026-11-08 09:00:00 UTC, the import date.
pub(crate) const NOW: i64 = 1_794_128_400;
/// A namespace no standard defines.
pub(crate) const UNKNOWN: &str = "http://example.com/camera/1.0/";

/// A RAF captured at `capture_time`, such as `2026:10:17 14:05:09`.
pub(crate) fn raw_taken_at(capture_time: &str) -> Vec<u8> {
    let exif = Tiff::new(ByteOrder::LittleEndian)
        .with_exif_text(DATE_TIME_ORIGINAL, capture_time)
        .to_bytes();
    raf(&jpeg(&exif))
}

/// A card with `files` `(name, content)`, all modified at the end of 2025, and the
/// empty destinations.
pub(crate) fn card_with(files: &[(&str, &[u8])]) -> MemoryFileSystem {
    let file_system = MemoryFileSystem::new();
    for folder in [SOURCE, DESTINATION].into_iter().chain(BACKUPS) {
        file_system.add_folder(Path::new(folder));
    }
    for (name, content) in files {
        put(&file_system, &Path::new(SOURCE).join(name), content);
    }
    file_system
}

pub(crate) fn put(file_system: &MemoryFileSystem, path: &Path, content: &[u8]) {
    file_system.add_file(path, content, Timestamp::from_unix_seconds(MODIFIED_2025));
}

pub(crate) fn lisbon() -> JobCode {
    JobCode::parse("Lisbon").unwrap()
}

/// The values of IMPORT-019.1.
pub(crate) fn n16_values() -> ImportMetadata {
    ImportMetadata {
        creator: vec!["Andy Pollari".to_owned()],
        copyright: Some("© {year} Andy Pollari".to_owned()),
        keywords: vec![Keyword::parse("Holiday/Beach").unwrap()],
        sidecar_naming: SidecarNaming::OriginalWithExtension,
    }
}

/// How one import is set up.
pub(crate) struct Import {
    template: NameTemplate,
    pub(crate) job_code: Option<JobCode>,
    pub(crate) metadata: ImportMetadata,
    backups: Vec<PathBuf>,
    pub(crate) policy: ConflictPolicy,
    pub(crate) dry_run: bool,
}

impl Import {
    /// An import with `template` and the job code `Lisbon`.
    pub(crate) fn lisbon(template: &str) -> Self {
        Import {
            template: NameTemplate::parse(template).unwrap(),
            job_code: Some(lisbon()),
            metadata: ImportMetadata::default(),
            backups: Vec::new(),
            policy: ConflictPolicy::AddSuffix,
            dry_run: false,
        }
    }

    pub(crate) fn backed_up(mut self) -> Self {
        self.backups = BACKUPS.iter().map(PathBuf::from).collect();
        self
    }

    pub(crate) fn settings(&self) -> ImportSettings<'_> {
        ImportSettings {
            source: Path::new(SOURCE),
            destination: Path::new(DESTINATION),
            backup_destinations: &self.backups,
            video_destination: None,
            video_backup_destinations: &[],
            template: &self.template,
            video_template: None,
            job_code: self.job_code.as_ref(),
            metadata: &self.metadata,
            mode: CompatibilityMode::On,
            policy: self.policy,
            dry_run: self.dry_run,
        }
    }
}

/// What the imports of one test share, as on one machine.
pub(crate) struct Machine {
    pub(crate) history: MemoryImportHistory,
    counters: MemorySequenceCounters,
    session: SessionSequence,
    pub(crate) clock: ManualClock,
    pub(crate) xml: RecordedXml,
    free_space: MemoryFreeSpace,
}

impl Machine {
    pub(crate) fn new() -> Self {
        Self::with_free_space(MemoryFreeSpace::unlimited())
    }

    pub(crate) fn with_free_space(free_space: MemoryFreeSpace) -> Self {
        Machine {
            history: MemoryImportHistory::new(),
            counters: MemorySequenceCounters::new(),
            session: SessionSequence::new(),
            clock: ManualClock::new(
                Timestamp::from_unix_seconds(NOW),
                UtcOffset::from_seconds(0).unwrap(),
            ),
            xml: RecordedXml::new(),
            free_space,
        }
    }

    pub(crate) fn import(&self, file_system: &MemoryFileSystem, import: &Import) -> ImportSummary {
        self.run(file_system)
            .with_xml_syntax(&self.xml)
            .run(&import.settings(), &mut NoProgress)
            .unwrap()
    }

    /// The import, with the error that stops it before the first copy.
    pub(crate) fn try_import(
        &self,
        file_system: &MemoryFileSystem,
        import: &Import,
    ) -> Result<ImportSummary, RunError> {
        self.run(file_system)
            .with_xml_syntax(&self.xml)
            .run(&import.settings(), &mut NoProgress)
    }

    /// The import through a run that was given no XML syntax.
    pub(crate) fn import_without_xml(
        &self,
        file_system: &MemoryFileSystem,
        import: &Import,
    ) -> Result<ImportSummary, RunError> {
        self.run(file_system)
            .run(&import.settings(), &mut NoProgress)
    }

    fn run<'a>(&'a self, file_system: &'a MemoryFileSystem) -> ImportRun<'a> {
        ImportRun::new(
            file_system,
            &FnvHasher,
            &self.clock,
            &self.counters,
            &self.session,
            &self.history,
            &self.free_space,
        )
    }

    /// The sidecar at `path`, parsed.
    pub(crate) fn sidecar(&self, file_system: &MemoryFileSystem, path: &str) -> XmpPacket {
        let content = file_system
            .content(Path::new(path))
            .unwrap_or_else(|| panic!("no sidecar at {path}"));
        parse_xmp(Path::new(path), &content, &self.xml).unwrap()
    }
}

pub(crate) fn report<'a>(summary: &'a ImportSummary, path: &str) -> &'a FileReport {
    summary
        .files
        .iter()
        .find(|file| file.path == Path::new(path))
        .unwrap_or_else(|| panic!("no report of {path} in {:?}", summary.files))
}

pub(crate) fn copied(report: &FileReport) -> &str {
    match &report.outcome {
        FileOutcome::Copied { target, .. } => target,
        other => panic!("copied expected, got {other:?}"),
    }
}

/// The `.xmp` files below `root`.
pub(crate) fn sidecars_below(file_system: &MemoryFileSystem, root: &str) -> Vec<PathBuf> {
    file_system
        .paths()
        .into_iter()
        .filter(|path| path.starts_with(root))
        .filter(|path| path.extension().is_some_and(|extension| extension == "xmp"))
        .collect()
}

pub(crate) fn photoshop(local: &str) -> XmpName {
    XmpName::new(metadata::xmp::namespaces::PHOTOSHOP, local)
}

/// The text of the simple property `name`.
pub(crate) fn text(packet: &XmpPacket, name: &XmpName) -> Option<String> {
    match &packet.property(name)?.value {
        XmpValue::Text(text) => Some(text.clone()),
        _ => None,
    }
}

/// A source sidecar with a property of a namespace no standard defines.
pub(crate) fn camera_packet() -> XmpPacket {
    let mut packet = XmpPacket::new();
    packet
        .set_text(&XmpName::new(UNKNOWN, "FilmMode"), "Classic Chrome")
        .unwrap();
    packet
}

/// The bytes of `packet` as the machine's XML syntax writes them.
pub(crate) fn bytes_of(machine: &Machine, packet: &XmpPacket) -> Vec<u8> {
    serialize_xmp(packet, &machine.clock, &machine.xml)
        .unwrap()
        .into_bytes()
}

/// A card with `DSCF0042.RAF` captured on 2026-10-17 and `extra` files.
pub(crate) fn raw_card_with(extra: &[(&str, &[u8])]) -> MemoryFileSystem {
    let raw = raw_taken_at("2026:10:17 14:05:09");
    let mut files: Vec<(&str, &[u8])> = vec![("DSCF0042.RAF", &raw)];
    files.extend_from_slice(extra);
    card_with(&files)
}
