// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! A source on a file system of its own, such as an opened camera (ADR 0034), read
//! only through `ReadOnlyFileSystem`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::{Path, PathBuf};

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{FnvHasher, MemoryCameras, MemoryFileSystem, MemoryFreeSpace};
use files::{Cameras, ReadOnlyFileSystem};
use history::testing::MemoryImportHistory;
use import::{
    ConflictPolicy, FileOutcome, ImportRun, ImportSettings, ImportSummary, NoProgress,
    SessionSequence,
};
use naming::testing::MemorySequenceCounters;
use naming::{CompatibilityMode, NameTemplate};

const PORT: &str = "usb:002,007";
const SOURCE: &str = "/store_00010001/DCIM";
const DESTINATION: &str = "/library";
/// 2026-10-17 14:05:09 UTC.
const MODIFIED: i64 = 1_792_245_909;
/// 2026-10-31 09:00:00 UTC.
const NOW: i64 = 1_793_437_200;

fn camera_with(files: &[&str]) -> MemoryFileSystem {
    let storage = MemoryFileSystem::new();
    for file in files {
        let path = Path::new(SOURCE).join(file);
        storage.add_file(
            &path,
            file.as_bytes(),
            Timestamp::from_unix_seconds(MODIFIED),
        );
    }
    storage
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

/// Imports `source` into `destinations` with `{name}.{ext}`, reading the source only
/// through `source` and keeping the import history in `history`.
fn import(
    source: &dyn ReadOnlyFileSystem,
    destinations: &MemoryFileSystem,
    history: &MemoryImportHistory,
) -> ImportSummary {
    let clock = ManualClock::new(
        Timestamp::from_unix_seconds(NOW),
        UtcOffset::from_seconds(0).unwrap(),
    );
    let template = NameTemplate::parse("{name}.{ext}").unwrap();
    let settings = ImportSettings {
        source: Path::new(SOURCE),
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
    ImportRun::new(
        destinations,
        &FnvHasher,
        &clock,
        counters,
        session,
        history,
        space,
    )
    .with_source(source)
    .run(&settings, &mut NoProgress)
    .unwrap()
}

#[test]
fn a_source_on_its_own_file_system_is_copied_to_the_destinations() {
    let storage = camera_with(&["100_FUJI/DSCF0001.RAF"]);
    let cameras = MemoryCameras::new().with_camera("Fujifilm X-T5", PORT, &storage);
    let camera = cameras.open(PORT).unwrap();
    let destinations = MemoryFileSystem::new();
    let before = snapshot(&storage);

    let summary = import(camera.as_ref(), &destinations, &MemoryImportHistory::new());

    assert!(
        matches!(&summary.files[0].outcome, FileOutcome::Copied { target, .. } if target == "DSCF0001.RAF"),
        "{summary:?}"
    );
    assert_eq!(
        destinations.content(&Path::new(DESTINATION).join("DSCF0001.RAF")),
        Some(b"100_FUJI/DSCF0001.RAF".to_vec())
    );
    assert!(
        !destinations
            .paths()
            .iter()
            .any(|path| path.starts_with(SOURCE))
    );
    assert_eq!(snapshot(&storage), before);
}

#[test]
fn a_source_on_its_own_file_system_is_recognized_the_second_time() {
    let storage = camera_with(&["100_FUJI/DSCF0001.RAF"]);
    let cameras = MemoryCameras::new().with_camera("Fujifilm X-T5", PORT, &storage);
    let camera = cameras.open(PORT).unwrap();
    let (destinations, history) = (MemoryFileSystem::new(), MemoryImportHistory::new());
    import(camera.as_ref(), &destinations, &history);

    let again = import(camera.as_ref(), &destinations, &history);

    assert!(
        matches!(&again.files[0].outcome, FileOutcome::ImportedBefore { .. }),
        "{again:?}"
    );
}
