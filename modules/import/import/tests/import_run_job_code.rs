// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! An import run with a job code (IMPORT-006), against `MemoryFileSystem`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::Path;

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{FnvHasher, MemoryFileSystem};
use import::{
    ConflictPolicy, FileOutcome, ImportRun, ImportSettings, ImportSummary, JobCode, MissingJobCode,
    NoProgress, RunError,
};
use naming::{CompatibilityMode, NameTemplate};

const SOURCE: &str = "/media/card";
const DESTINATION: &str = "/library";
/// 2026-10-31 09:00:00 UTC.
const NOW: i64 = 1_793_437_200;

fn source_with(file_name: &str) -> MemoryFileSystem {
    let file_system = MemoryFileSystem::new();
    file_system.add_folder(Path::new(SOURCE));
    let modified = Timestamp::from_unix_seconds(NOW);
    file_system.add_file(
        &Path::new(SOURCE).join(file_name),
        file_name.as_bytes(),
        modified,
    );
    file_system
}

fn import(
    file_system: &MemoryFileSystem,
    template: &str,
    job_code: Option<&str>,
) -> Result<ImportSummary, RunError> {
    let clock = ManualClock::new(
        Timestamp::from_unix_seconds(NOW),
        UtcOffset::from_seconds(0).unwrap(),
    );
    let template = NameTemplate::parse(template).unwrap();
    let job_code = job_code.map(|text| JobCode::parse(text).unwrap());
    let settings = ImportSettings {
        source: Path::new(SOURCE),
        destination: Path::new(DESTINATION),
        template: &template,
        job_code: job_code.as_ref(),
        mode: CompatibilityMode::On,
        policy: ConflictPolicy::AddSuffix,
        dry_run: false,
    };
    ImportRun::new(file_system, &FnvHasher, &clock).run(&settings, &mut NoProgress)
}

fn copied_target(summary: &ImportSummary) -> Option<&str> {
    match &summary.files.first()?.outcome {
        FileOutcome::Copied { target, .. } => Some(target),
        _ => None,
    }
}

#[test]
fn the_job_code_is_part_of_the_target() {
    let file_system = source_with("a.MOV");

    let summary = import(&file_system, "{job}/{name}.{ext}", Some("Smith wedding")).unwrap();

    assert_eq!(copied_target(&summary), Some("Smith wedding/a.MOV"));
}

/// spec: IMPORT-006.6
#[test]
fn a_template_with_job_and_no_job_code_stops_the_run() {
    let file_system = source_with("a.MOV");
    let paths_before = file_system.paths();

    let error = import(&file_system, "{job}/{name}.{ext}", None).unwrap_err();

    assert!(matches!(error, RunError::MissingJobCode(MissingJobCode)));
    assert!(error.to_string().contains("{job}"), "{error}");
    assert_eq!(file_system.paths(), paths_before);
}

/// spec: IMPORT-006.7
#[test]
fn a_job_code_without_job_in_the_template_changes_nothing() {
    let file_system = source_with("DSC_0042.RAF");

    let summary = import(&file_system, "{name}.{ext}", Some("Smith wedding")).unwrap();

    assert_eq!(copied_target(&summary), Some("DSC_0042.RAF"));
}
