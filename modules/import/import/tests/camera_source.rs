// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Cameras and phones as a source (IMPORT-014): only `DCIM` of each storage is
//! scanned, and the camera is imported like a card, read only through an opened
//! `MemoryCameras` camera.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::{Path, PathBuf};

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{FnvHasher, MemoryCameras, MemoryFileSystem, MemoryFreeSpace};
use files::{Cameras, ReadOnlyFileSystem};
use history::testing::MemoryImportHistory;
use import::{
    CAMERA_FORMAT_READINESS_NOTE, ConflictPolicy, FileOutcome, FileReport, FormatReadiness,
    ImportRun, ImportSettings, ImportSummary, LeftOut, LeftOutReason, NoProgress, ProgressSink,
    ScanError, SessionSequence, SourceScan, scan_camera,
};
use naming::testing::MemorySequenceCounters;
use naming::{CompatibilityMode, NameTemplate};

const PORT: &str = "usb:002,007";
const ROOT: &str = "/";
const DESTINATION: &str = "/library";
/// 2026-10-17 14:05:09 UTC.
const MODIFIED: i64 = 1_792_245_909;
/// 2026-10-31 09:00:00 UTC.
const NOW: i64 = 1_793_437_200;

/// A camera file system holding `files`, given relative to its root; each file holds
/// its own path.
fn camera_with(files: &[&str]) -> MemoryFileSystem {
    let camera = MemoryFileSystem::new();
    for file in files {
        camera.add_file(
            &Path::new(ROOT).join(file),
            file.as_bytes(),
            Timestamp::from_unix_seconds(MODIFIED),
        );
    }
    camera
}

/// Opens `camera` through `MemoryCameras`.
fn open(camera: &MemoryFileSystem) -> Box<dyn ReadOnlyFileSystem + Send> {
    let cameras = MemoryCameras::new().with_camera("Fujifilm X-T5", PORT, camera);
    cameras.open(PORT).unwrap()
}

/// Opens `camera` and scans it.
fn scan(camera: &MemoryFileSystem) -> SourceScan {
    scan_camera(open(camera).as_ref(), Path::new(ROOT)).unwrap()
}

fn found_paths(scan: &SourceScan) -> Vec<PathBuf> {
    scan.files.iter().map(|file| file.path.clone()).collect()
}

/// Everything on `file_system`, with content and modification time.
fn snapshot(file_system: &MemoryFileSystem) -> Vec<(PathBuf, Option<Vec<u8>>, Option<Timestamp>)> {
    file_system
        .paths()
        .into_iter()
        .map(|path| {
            let content = file_system.content(&path);
            let modified = file_system.modified(&path);
            (path, content, modified)
        })
        .collect()
}

/// Imports the opened camera `source` with its `scan` into `destinations` with
/// `{name}.{ext}`, as a caller does: it plans, starts and executes the run.
fn import(
    source: &dyn ReadOnlyFileSystem,
    scan: &SourceScan,
    destinations: &MemoryFileSystem,
    history: &MemoryImportHistory,
    progress: &mut dyn ProgressSink,
) -> ImportSummary {
    let clock = ManualClock::new(
        Timestamp::from_unix_seconds(NOW),
        UtcOffset::from_seconds(0).unwrap(),
    );
    let template = NameTemplate::parse("{name}.{ext}").unwrap();
    let settings = ImportSettings {
        source: Path::new(ROOT),
        destination: Path::new(DESTINATION),
        backup_destinations: &[],
        video_destination: None,
        video_backup_destinations: &[],
        template: &template,
        video_template: None,
        job_code: None,
        mode: CompatibilityMode::On,
        policy: ConflictPolicy::AddSuffix,
        dry_run: false,
    };
    let (counters, session) = (&MemorySequenceCounters::new(), &SessionSequence::new());
    let space = &MemoryFreeSpace::unlimited();
    let run = ImportRun::new(
        destinations,
        &FnvHasher,
        &clock,
        counters,
        session,
        history,
        space,
    )
    .with_source(source);
    let plan = run.plan(&settings, scan).unwrap();
    let plan = run.start(&settings, plan).unwrap();
    run.execute(&settings, plan, progress)
}

/// Opens `camera` and imports it into `destinations`.
fn import_camera(
    camera: &MemoryFileSystem,
    destinations: &MemoryFileSystem,
    history: &MemoryImportHistory,
) -> ImportSummary {
    let opened = open(camera);
    let scan = scan_camera(opened.as_ref(), Path::new(ROOT)).unwrap();
    import(
        opened.as_ref(),
        &scan,
        destinations,
        history,
        &mut NoProgress,
    )
}

/// Stops the camera from answering once the first file is finished, as when it is
/// unplugged during the run.
struct UnplugAfterFirstFile<'a> {
    camera: &'a MemoryFileSystem,
}

impl ProgressSink for UnplugAfterFirstFile<'_> {
    fn scan_finished(&mut self, _scan: &SourceScan) {}

    fn file_started(&mut self, _path: &Path) {}

    fn file_finished(&mut self, _report: &FileReport) {
        self.camera.stop_after_operations(0);
    }
}

/// spec: IMPORT-014.1
#[test]
fn files_below_dcim_are_found_with_their_storage_folder() {
    let camera = camera_with(&["store_00010001/DCIM/100_FUJI/DSCF0001.RAF"]);

    let scan = scan(&camera);

    assert_eq!(
        found_paths(&scan),
        [PathBuf::from("store_00010001/DCIM/100_FUJI/DSCF0001.RAF")]
    );
}

/// spec: IMPORT-014.2
#[test]
fn files_outside_dcim_are_not_looked_at() {
    let camera = camera_with(&[
        "store_00010001/DCIM/100_FUJI/DSCF0001.RAF",
        "store_00010001/Music/song.mp4",
        "store_00010001/Download/photo.jpg",
    ]);

    let scan = scan(&camera);

    assert_eq!(
        found_paths(&scan),
        [PathBuf::from("store_00010001/DCIM/100_FUJI/DSCF0001.RAF")]
    );
    assert_eq!(scan.left_out, []);
}

/// spec: IMPORT-014.3
#[test]
fn every_storage_is_scanned_in_byte_order() {
    let camera = camera_with(&[
        "store_00020001/DCIM/100_FUJI/DSCF0002.RAF",
        "store_00010001/DCIM/100_FUJI/DSCF0001.RAF",
    ]);

    let scan = scan(&camera);

    assert_eq!(
        found_paths(&scan),
        [
            PathBuf::from("store_00010001/DCIM/100_FUJI/DSCF0001.RAF"),
            PathBuf::from("store_00020001/DCIM/100_FUJI/DSCF0002.RAF"),
        ]
    );
}

/// spec: IMPORT-014.4
#[test]
fn a_storage_without_dcim_gives_no_files() {
    let camera = camera_with(&["store_00010001/Download/photo.jpg"]);

    let scan = scan(&camera);

    assert_eq!(scan, SourceScan::default());
}

/// spec: IMPORT-014.11
#[test]
fn a_camera_without_a_storage_is_an_error() {
    let error = scan_camera(open(&MemoryFileSystem::new()).as_ref(), Path::new(ROOT));

    assert!(matches!(error, Err(ScanError::NoStorage)), "{error:?}");
}

/// spec: IMPORT-014.5
#[test]
fn the_scan_rules_of_a_folder_apply_below_dcim() {
    let camera = camera_with(&[
        "store_00010001/DCIM/100_FUJI/DSCF0001.RAF",
        "store_00010001/DCIM/100_FUJI/._DSCF0001.RAF",
        "store_00010001/DCIM/100_FUJI/notes.txt",
    ]);

    let scan = scan(&camera);

    assert_eq!(
        found_paths(&scan),
        [PathBuf::from("store_00010001/DCIM/100_FUJI/DSCF0001.RAF")]
    );
    assert_eq!(
        scan.left_out,
        [
            LeftOut {
                path: PathBuf::from("store_00010001/DCIM/100_FUJI/._DSCF0001.RAF"),
                reason: LeftOutReason::Hidden,
            },
            LeftOut {
                path: PathBuf::from("store_00010001/DCIM/100_FUJI/notes.txt"),
                reason: LeftOutReason::UnsupportedType,
            },
        ]
    );
}

/// spec: IMPORT-014.12
#[test]
fn a_dcim_that_cannot_be_looked_at_is_left_out() {
    let camera = camera_with(&["store_00010001/DCIM/100_FUJI/DSCF0001.RAF"]);
    let opened = open(&camera);
    // Listing the root succeeds; looking at the storage's DCIM does not.
    camera.stop_after_operations(2);

    let scan = scan_camera(opened.as_ref(), Path::new(ROOT)).unwrap();

    assert_eq!(scan.files, []);
    assert!(
        matches!(
            &scan.left_out[..],
            [LeftOut { path, reason: LeftOutReason::CannotBeRead(_) }]
                if path == Path::new("store_00010001/DCIM")
        ),
        "{scan:?}"
    );
    let summary = import(
        opened.as_ref(),
        &scan,
        &MemoryFileSystem::new(),
        &MemoryImportHistory::new(),
        &mut NoProgress,
    );
    assert!(!FormatReadiness::of(&summary).can_be_formatted());
}

/// spec: IMPORT-014.6
#[test]
fn a_camera_import_is_copied_verified_and_recorded() {
    let camera = camera_with(&["store_00010001/DCIM/100_FUJI/DSCF0001.RAF"]);
    let (destinations, history) = (MemoryFileSystem::new(), MemoryImportHistory::new());

    let summary = import_camera(&camera, &destinations, &history);

    assert!(
        matches!(&summary.files[..], [file] if matches!(&file.outcome, FileOutcome::Copied { target, .. } if target == "DSCF0001.RAF")),
        "{summary:?}"
    );
    assert_eq!(
        destinations.content(&Path::new(DESTINATION).join("DSCF0001.RAF")),
        Some(b"store_00010001/DCIM/100_FUJI/DSCF0001.RAF".to_vec())
    );
    let copies = history.copies();
    assert_eq!(copies.len(), 1);
    assert_eq!(
        copies[0].source_path,
        Path::new("store_00010001/DCIM/100_FUJI/DSCF0001.RAF")
    );
}

/// spec: IMPORT-014.7
#[test]
fn the_camera_is_not_changed() {
    let camera = camera_with(&["store_00010001/DCIM/100_FUJI/DSCF0001.RAF"]);
    let before = snapshot(&camera);

    let summary = import_camera(
        &camera,
        &MemoryFileSystem::new(),
        &MemoryImportHistory::new(),
    );

    assert_eq!(summary.files.len(), 1, "{summary:?}");
    assert_eq!(snapshot(&camera), before);
}

/// spec: IMPORT-014.8
#[test]
fn a_second_import_copies_nothing() {
    let camera = camera_with(&["store_00010001/DCIM/100_FUJI/DSCF0001.RAF"]);
    let (destinations, history) = (MemoryFileSystem::new(), MemoryImportHistory::new());
    import_camera(&camera, &destinations, &history);
    let after_first = snapshot(&destinations);

    let again = import_camera(&camera, &destinations, &history);

    assert!(
        matches!(&again.files[..], [file] if matches!(file.outcome, FileOutcome::ImportedBefore { .. })),
        "{again:?}"
    );
    assert_eq!(snapshot(&destinations), after_first);
}

/// spec: IMPORT-014.9
#[test]
fn a_camera_that_goes_away_fails_the_remaining_files() {
    let camera = camera_with(&[
        "store_00010001/DCIM/100_FUJI/DSCF0001.RAF",
        "store_00010001/DCIM/100_FUJI/DSCF0002.RAF",
    ]);
    let opened = open(&camera);
    let mut unplug = UnplugAfterFirstFile { camera: &camera };

    let scan = scan_camera(opened.as_ref(), Path::new(ROOT)).unwrap();
    let summary = import(
        opened.as_ref(),
        &scan,
        &MemoryFileSystem::new(),
        &MemoryImportHistory::new(),
        &mut unplug,
    );

    assert!(
        matches!(&summary.files[0].outcome, FileOutcome::Copied { .. }),
        "{summary:?}"
    );
    assert!(
        matches!(
            &summary.files[1].outcome,
            FileOutcome::Failed(error) if error.to_string().contains("the program stopped")
        ),
        "{summary:?}"
    );
    assert_eq!(summary.files.len(), 2);
    assert!(!summary.stopped);
    assert!(!FormatReadiness::of(&summary).can_be_formatted());
}

/// spec: IMPORT-014.10
#[test]
fn a_finished_camera_import_is_ready_to_be_formatted_whatever_lies_outside_dcim() {
    let camera = camera_with(&[
        "store_00010001/DCIM/100_FUJI/DSCF0001.RAF",
        "store_00010001/Music/song.mp4",
        "store_00010001/Download/photo.jpg",
    ]);

    let summary = import_camera(
        &camera,
        &MemoryFileSystem::new(),
        &MemoryImportHistory::new(),
    );

    assert_eq!(summary.files.len(), 1, "{summary:?}");
    let readiness = FormatReadiness::of(&summary);
    assert!(readiness.can_be_formatted(), "{readiness:?}");
    assert_eq!(readiness.left_out_not_saved, 0);
}

/// spec: IMPORT-014.10
#[test]
fn the_format_readiness_of_a_camera_notes_that_only_dcim_was_checked() {
    assert_eq!(
        CAMERA_FORMAT_READINESS_NOTE,
        "Only the files below DCIM were checked. \
         Files outside DCIM, such as a phone's other folders, are not backed up."
    );
}
