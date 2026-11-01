// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import run (ADR 0020 §1, IMPORT-004.11), against `MemoryFileSystem`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::{Path, PathBuf};

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{Fault, FnvHasher, MemoryFileSystem};
use import::{
    ConflictPolicy, FileError, FileOutcome, FileReport, ImportRun, ImportSettings, ImportSummary,
    NoProgress, ProgressSink, RunError, ScanError, SourceScan,
};
use metadata::NoCaptureTime;
use metadata::testing::{ByteOrder, DATE_TIME_ORIGINAL, Tiff, jpeg};
use naming::{CompatibilityMode, NameTemplate};

const SOURCE: &str = "/media/card";
const DESTINATION: &str = "/library";
const TEMPLATE: &str = "{date:%Y-%m-%d}/{name}.{ext}";

/// 2026-10-17 14:05:09 UTC.
const MODIFIED: i64 = 1_792_245_909;
/// 2026-10-17 23:30:00 UTC: 2026-10-18 in UTC+2.
const MODIFIED_LATE: i64 = 1_792_279_800;
/// 2026-10-31 09:00:00 UTC.
const NOW: i64 = 1_793_437_200;
const TWO_HOURS: i32 = 7_200;

fn source_with(files: &[&str]) -> MemoryFileSystem {
    let file_system = MemoryFileSystem::new();
    file_system.add_folder(Path::new(SOURCE));
    for file in files {
        add(&file_system, file, file.as_bytes(), MODIFIED);
    }
    file_system
}

fn add(file_system: &MemoryFileSystem, relative: &str, content: &[u8], modified: i64) {
    let modified = Timestamp::from_unix_seconds(modified);
    file_system.add_file(&Path::new(SOURCE).join(relative), content, modified);
}

fn photo_taken_at(capture_time: &str) -> Vec<u8> {
    let exif = Tiff::new(ByteOrder::LittleEndian)
        .with_exif_text(DATE_TIME_ORIGINAL, capture_time)
        .to_bytes();
    jpeg(&exif)
}

struct Setup {
    template: &'static str,
    policy: ConflictPolicy,
    dry_run: bool,
    offset: i32,
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
    fn on(&self, file_system: &MemoryFileSystem, progress: &mut dyn ProgressSink) -> ImportSummary {
        self.try_on(file_system, Path::new(DESTINATION), progress)
            .unwrap()
    }

    fn try_on(
        &self,
        file_system: &MemoryFileSystem,
        destination: &Path,
        progress: &mut dyn ProgressSink,
    ) -> Result<ImportSummary, RunError> {
        let clock = ManualClock::new(
            Timestamp::from_unix_seconds(NOW),
            UtcOffset::from_seconds(self.offset).unwrap(),
        );
        let template = NameTemplate::parse(self.template).unwrap();
        let settings = ImportSettings {
            source: Path::new(SOURCE),
            destination,
            template: &template,
            job_code: None,
            mode: CompatibilityMode::On,
            policy: self.policy,
            dry_run: self.dry_run,
        };
        ImportRun::new(file_system, &FnvHasher, &clock).run(&settings, progress)
    }
}

fn import(file_system: &MemoryFileSystem) -> ImportSummary {
    Setup::default().on(file_system, &mut NoProgress)
}

/// The target of a file that was copied or would be copied.
fn target(report: &FileReport) -> Option<&str> {
    match &report.outcome {
        FileOutcome::Copied { target, .. } | FileOutcome::WouldCopy { target } => Some(target),
        _ => None,
    }
}

fn failure(report: &FileReport) -> Option<&FileError> {
    match &report.outcome {
        FileOutcome::Failed(error) => Some(error),
        _ => None,
    }
}

fn copied_content(file_system: &MemoryFileSystem, target: &str) -> Option<Vec<u8>> {
    file_system.content(&Path::new(DESTINATION).join(target))
}

/// spec: IMPORT-004.11
#[test]
fn one_failed_file_does_not_stop_the_import() {
    let file_system = source_with(&["a.MOV", "b.MOV", "c.MOV"]);
    let second = Path::new(SOURCE).join("b.MOV");
    file_system.inject_fault(&second, Fault::ReadFails { after_bytes: 2 });

    let summary = import(&file_system);

    let [first, second, third]: &[FileReport; 3] = summary.files[..].try_into().unwrap();
    assert_eq!(target(first), Some("2026-10-17/a.MOV"));
    assert_eq!(target(third), Some("2026-10-17/c.MOV"));
    assert!(matches!(failure(second).unwrap(), FileError::Copy(_)));
    assert_eq!(
        copied_content(&file_system, "2026-10-17/a.MOV").unwrap(),
        b"a.MOV"
    );
    assert_eq!(copied_content(&file_system, "2026-10-17/b.MOV"), None);
    assert_eq!(
        copied_content(&file_system, "2026-10-17/c.MOV").unwrap(),
        b"c.MOV"
    );
    assert!(summary.has_failures());
}

#[test]
fn a_photo_that_cannot_be_read_fails_before_it_is_copied() {
    let file_system = source_with(&["DSC_0001.JPG", "DSC_0002.JPG"]);
    let broken = Path::new(SOURCE).join("DSC_0001.JPG");
    file_system.inject_fault(&broken, Fault::ReadFails { after_bytes: 0 });

    let summary = import(&file_system);

    let error = failure(&summary.files[0]).unwrap();
    assert!(matches!(error, FileError::CannotBeRead { .. }));
    assert!(error.to_string().starts_with("cannot be read: "), "{error}");
    assert_eq!(target(&summary.files[1]), Some("2026-10-17/DSC_0002.JPG"));
}

#[test]
fn a_photo_is_named_by_its_capture_time() {
    let file_system = source_with(&[]);
    add(
        &file_system,
        "DSC_0001.JPG",
        &photo_taken_at("2026:10:12 08:30:00"),
        MODIFIED,
    );

    let summary = import(&file_system);

    assert_eq!(target(&summary.files[0]), Some("2026-10-12/DSC_0001.JPG"));
    assert_eq!(summary.files[0].dated_by_modification_time, None);
}

#[test]
fn a_file_without_capture_time_is_named_by_its_local_modification_time() {
    let file_system = source_with(&[]);
    add(&file_system, "clip.MOV", b"video", MODIFIED_LATE);
    add(
        &file_system,
        "DSC_0001.JPG",
        &photo_taken_at("    "),
        MODIFIED_LATE,
    );
    let run = Setup {
        offset: TWO_HOURS,
        ..Setup::default()
    };

    let summary = run.on(&file_system, &mut NoProgress);

    let [photo, clip]: &[FileReport; 2] = summary.files[..].try_into().unwrap();
    assert_eq!(target(photo), Some("2026-10-18/DSC_0001.JPG"));
    assert_eq!(
        photo.dated_by_modification_time,
        Some(NoCaptureTime::Missing)
    );
    assert_eq!(target(clip), Some("2026-10-18/clip.MOV"));
    assert_eq!(
        clip.dated_by_modification_time,
        Some(NoCaptureTime::FormatNotRead)
    );
}

#[test]
fn the_import_time_is_local_time() {
    let file_system = source_with(&["clip.MOV"]);
    let run = Setup {
        template: "{imported:%Y-%m-%d %H}/{name}.{ext}",
        offset: TWO_HOURS,
        ..Setup::default()
    };

    let summary = run.on(&file_system, &mut NoProgress);

    assert_eq!(target(&summary.files[0]), Some("2026-10-31 11/clip.MOV"));
}

#[test]
fn a_second_run_copies_nothing() {
    let file_system = source_with(&["a.MOV", "b.MOV"]);
    import(&file_system);
    let paths_after_first_run = file_system.paths();

    let summary = import(&file_system);

    assert!(
        summary
            .files
            .iter()
            .all(|report| matches!(report.outcome, FileOutcome::AlreadyImported { .. }))
    );
    assert_eq!(file_system.paths(), paths_after_first_run);
}

#[test]
fn the_fail_policy_fails_only_the_conflicting_file() {
    let file_system = source_with(&["a.MOV", "b.MOV"]);
    let taken = Path::new(DESTINATION).join("2026-10-17/a.MOV");
    file_system.add_file(&taken, b"another clip", Timestamp::UNIX_EPOCH);
    let run = Setup {
        policy: ConflictPolicy::Fail,
        ..Setup::default()
    };

    let summary = run.on(&file_system, &mut NoProgress);

    let error = failure(&summary.files[0]).unwrap();
    assert_eq!(
        error.to_string(),
        "name conflict: 2026-10-17/a.MOV holds a different file"
    );
    assert_eq!(file_system.content(&taken).unwrap(), b"another clip");
    assert_eq!(target(&summary.files[1]), Some("2026-10-17/b.MOV"));
}

#[test]
fn the_skip_policy_and_duplicates_copy_nothing() {
    let file_system = source_with(&["a.MOV"]);
    add(&file_system, "copy/a.MOV", b"a.MOV", MODIFIED);
    add(&file_system, "other/a.MOV", b"different", MODIFIED);
    let run = Setup {
        policy: ConflictPolicy::Skip,
        ..Setup::default()
    };

    let summary = run.on(&file_system, &mut NoProgress);

    let outcomes: Vec<&FileOutcome> = summary.files.iter().map(|file| &file.outcome).collect();
    assert!(matches!(outcomes[0], FileOutcome::Copied { .. }));
    assert!(matches!(
        outcomes[1],
        FileOutcome::DuplicateOf { earlier } if earlier == Path::new("a.MOV")
    ));
    assert!(matches!(
        outcomes[2],
        FileOutcome::SkippedNameConflict { target } if target == "2026-10-17/a.MOV"
    ));
}

#[test]
fn a_file_that_cannot_be_named_fails() {
    let file_system = source_with(&["clip.MOV", "DSC_0001.MOV"]);
    let run = Setup {
        template: "{date:%Y}/IMG_{number}.{ext}",
        ..Setup::default()
    };

    let summary = run.on(&file_system, &mut NoProgress);

    assert_eq!(target(&summary.files[0]), Some("2026/IMG_0001.MOV"));
    assert!(matches!(
        failure(&summary.files[1]).unwrap(),
        FileError::CannotBeNamed(_)
    ));
}

#[test]
fn a_dry_run_writes_nothing() {
    let file_system = source_with(&["a.MOV"]);
    let paths_before = file_system.paths();
    let run = Setup {
        dry_run: true,
        ..Setup::default()
    };

    let summary = run.on(&file_system, &mut NoProgress);

    assert!(matches!(
        &summary.files[0].outcome,
        FileOutcome::WouldCopy { target } if target == "2026-10-17/a.MOV"
    ));
    assert_eq!(file_system.paths(), paths_before);
}

#[test]
fn left_out_files_are_in_the_summary() {
    let file_system = source_with(&["a.MOV", "notes.txt"]);

    let summary = import(&file_system);

    assert_eq!(summary.left_out.len(), 1);
    assert_eq!(summary.left_out[0].path, Path::new("notes.txt"));
    assert!(!summary.has_failures());
}

/// Records each step as text.
#[derive(Default)]
struct Recorder(Vec<String>);

impl ProgressSink for Recorder {
    fn scan_finished(&mut self, scan: &SourceScan) {
        let found = scan.files.len();
        let left_out = scan.left_out.len();
        self.0
            .push(format!("scanned {found} found, {left_out} left out"));
    }

    fn file_started(&mut self, path: &Path) {
        self.0.push(format!("started {}", path.display()));
    }

    fn file_finished(&mut self, report: &FileReport) {
        let result = match &report.outcome {
            FileOutcome::Failed(_) => "failed".to_owned(),
            _ => target(report).unwrap_or("not copied").to_owned(),
        };
        self.0
            .push(format!("finished {} {result}", report.path.display()));
    }
}

#[test]
fn each_step_is_reported_in_scan_order() {
    let file_system = source_with(&["b.MOV", "a.MOV", "notes.txt"]);
    let broken = Path::new(SOURCE).join("b.MOV");
    file_system.inject_fault(&broken, Fault::ReadFails { after_bytes: 0 });
    let mut recorder = Recorder::default();

    Setup::default().on(&file_system, &mut recorder);

    assert_eq!(
        recorder.0,
        [
            "scanned 2 found, 1 left out",
            "started a.MOV",
            "finished a.MOV 2026-10-17/a.MOV",
            "started b.MOV",
            "finished b.MOV failed",
        ]
    );
}

#[test]
fn a_source_that_cannot_be_scanned_stops_the_run() {
    let file_system = source_with(&["a.MOV"]);
    let paths_before = file_system.paths();
    let mut recorder = Recorder::default();
    let inside = PathBuf::from(SOURCE).join("library");

    let error = Setup::default()
        .try_on(&file_system, &inside, &mut recorder)
        .unwrap_err();

    assert!(matches!(
        error,
        RunError::Scan(ScanError::DestinationInsideSource { .. })
    ));
    assert_eq!(recorder.0, Vec::<String>::new());
    assert_eq!(file_system.paths(), paths_before);
}
