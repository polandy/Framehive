// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The destinations of a camera import (ADR 0034, CLI-013): the camera is a file
//! system of its own, so no destination is checked against its root.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::{Path, PathBuf};

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::Cameras;
use files::testing::{FnvHasher, MemoryCameras, MemoryFileSystem, MemoryFreeSpace};
use history::testing::MemoryImportHistory;
use import::{ConflictPolicy, ImportRun, ImportSettings, NoProgress, SessionSequence, scan_camera};
use naming::testing::MemorySequenceCounters;
use naming::{CompatibilityMode, NameTemplate};

const PORT: &str = "usb:002,007";
const ROOT: &str = "/";
const DESTINATION: &str = "/library";
/// 2026-10-17 14:05:09 UTC.
const MODIFIED: i64 = 1_792_245_909;
/// 2026-10-31 09:00:00 UTC.
const NOW: i64 = 1_793_437_200;

/// The root `/` of a camera contains every path of the computer, but no destination
/// can lie on the camera: backup and video destinations are not checked against it,
/// and the history names the run as the caller says.
#[test]
fn a_camera_takes_backup_and_video_destinations_and_a_source_name() {
    let camera = MemoryFileSystem::new();
    camera.add_file(
        Path::new("/store_00010001/DCIM/100_FUJI/DSCF0001.RAF"),
        b"raw",
        Timestamp::from_unix_seconds(MODIFIED),
    );
    let (destinations, history) = (MemoryFileSystem::new(), MemoryImportHistory::new());
    let cameras = MemoryCameras::new().with_camera("Fujifilm X-T5", PORT, &camera);
    let opened = cameras.open(PORT).unwrap();
    let clock = ManualClock::new(
        Timestamp::from_unix_seconds(NOW),
        UtcOffset::from_seconds(0).unwrap(),
    );
    let template = NameTemplate::parse("{name}.{ext}").unwrap();
    let backups = [PathBuf::from("/backup")];
    let settings = ImportSettings {
        source: Path::new(ROOT),
        destination: Path::new(DESTINATION),
        backup_destinations: &backups,
        video_destination: Some(Path::new("/videos")),
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
        &destinations,
        &FnvHasher,
        &clock,
        counters,
        session,
        &history,
        space,
    )
    .with_source(opened.as_ref())
    .with_source_name(Path::new("camera:Fujifilm X-T5"));
    let scan = scan_camera(opened.as_ref(), Path::new(ROOT)).unwrap();
    let plan = run.plan(&settings, &scan).unwrap();
    let plan = run.start(&settings, plan).unwrap();

    let summary = run.execute(&settings, plan, &mut NoProgress);

    assert!(!summary.has_failures(), "{summary:?}");
    assert!(
        destinations
            .content(Path::new("/backup/DSCF0001.RAF"))
            .is_some()
    );
    let runs = history::ImportHistory::list_runs(&history).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].start.source, Path::new("camera:Fujifilm X-T5"));
}
