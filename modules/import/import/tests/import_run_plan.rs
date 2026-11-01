// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Planning and executing an import run as separate steps (ADR 0020 §1), against
//! `MemoryFileSystem`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::path::Path;

use domain::testing::ManualClock;
use domain::{Timestamp, UtcOffset};
use files::testing::{Fault, FnvHasher, MemoryFileSystem};
use import::{
    ConflictPolicy, FileReport, ImportRun, ImportSettings, PlannedRun, ProgressSink, SourceScan,
    scan_source,
};
use naming::{CompatibilityMode, NameTemplate};

const SOURCE: &str = "/media/card";
const DESTINATION: &str = "/library";
/// 2026-10-17 14:05:09 UTC.
const MODIFIED: i64 = 1_792_245_909;
/// 2026-10-31 09:00:00 UTC.
const NOW: i64 = 1_793_437_200;

fn source_with(files: &[&str]) -> MemoryFileSystem {
    let file_system = MemoryFileSystem::new();
    file_system.add_folder(Path::new(SOURCE));
    for file in files {
        file_system.add_file(
            &Path::new(SOURCE).join(file),
            file.as_bytes(),
            Timestamp::from_unix_seconds(MODIFIED),
        );
    }
    file_system
}

fn clock() -> ManualClock {
    ManualClock::new(
        Timestamp::from_unix_seconds(NOW),
        UtcOffset::from_seconds(0).unwrap(),
    )
}

fn settings(template: &NameTemplate) -> ImportSettings<'_> {
    ImportSettings {
        source: Path::new(SOURCE),
        destination: Path::new(DESTINATION),
        template,
        job_code: None,
        mode: CompatibilityMode::On,
        policy: ConflictPolicy::AddSuffix,
        dry_run: false,
    }
}

fn template() -> NameTemplate {
    NameTemplate::parse("{date:%Y-%m-%d}/{name}.{ext}").unwrap()
}

fn scan(file_system: &MemoryFileSystem) -> SourceScan {
    scan_source(file_system, Path::new(SOURCE), Path::new(DESTINATION)).unwrap()
}

/// How many file system operations planning the first `count` files makes.
fn operations_to_plan(file_system: &MemoryFileSystem, count: usize) -> usize {
    let template = template();
    let clock = clock();
    let scan = scan(file_system);
    let before = file_system.operations();
    ImportRun::new(file_system, &FnvHasher, &clock)
        .plan_first(&settings(&template), &scan, count)
        .unwrap();
    file_system.operations() - before
}

fn targets(plan: &PlannedRun) -> Vec<&str> {
    plan.files()
        .iter()
        .map(|file| file.planned.as_ref().unwrap().target.as_deref().unwrap())
        .collect()
}

/// Records each progress step as a line of text.
#[derive(Default)]
struct Recorder(Vec<String>);

impl ProgressSink for Recorder {
    fn scan_finished(&mut self, scan: &SourceScan) {
        self.0.push(format!("scan {}", scan.files.len()));
    }
    fn file_started(&mut self, path: &Path) {
        self.0.push(format!("started {}", path.display()));
    }
    fn file_finished(&mut self, report: &FileReport) {
        self.0.push(format!("finished {report:?}"));
    }
}

#[test]
fn planning_the_first_files_reads_only_those() {
    let five = source_with(&["a.JPG", "b.JPG", "c.JPG", "d.JPG", "e.JPG"]);
    let two = source_with(&["a.JPG", "b.JPG"]);

    assert_eq!(operations_to_plan(&five, 2), operations_to_plan(&two, 2));
    assert!(operations_to_plan(&five, 0) < operations_to_plan(&five, 2));
}

#[test]
fn planning_the_first_files_names_them_and_writes_nothing() {
    let file_system = source_with(&["a.JPG", "b.JPG", "c.JPG"]);
    let paths_before = file_system.paths();
    let template = template();
    let clock = clock();

    let plan = ImportRun::new(&file_system, &FnvHasher, &clock)
        .plan_first(&settings(&template), &scan(&file_system), 2)
        .unwrap();

    assert_eq!(targets(&plan), ["2026-10-17/a.JPG", "2026-10-17/b.JPG"]);
    assert_eq!(file_system.paths(), paths_before);
}

#[test]
fn planning_more_files_than_the_scan_found_plans_all() {
    let file_system = source_with(&["a.JPG", "b.JPG"]);
    let template = template();
    let clock = clock();
    let scan = scan(&file_system);
    let run = ImportRun::new(&file_system, &FnvHasher, &clock);

    let first = run.plan_first(&settings(&template), &scan, 5).unwrap();
    let all = run.plan(&settings(&template), &scan).unwrap();

    assert_eq!(targets(&first), targets(&all));
}

#[test]
fn executing_the_plan_of_all_files_is_a_run() {
    // A JPEG that cannot be read fails while it is dated, a video only when copied.
    let files = ["a.JPG", "b.JPG", "broken.JPG", "broken.MOV", "notes.txt"];
    let by_run = source_with(&files);
    let by_steps = source_with(&files);
    for file_system in [&by_run, &by_steps] {
        for broken in ["broken.JPG", "broken.MOV"] {
            let broken = Path::new(SOURCE).join(broken);
            file_system.inject_fault(&broken, Fault::ReadFails { after_bytes: 0 });
        }
        file_system.add_file(
            &Path::new(DESTINATION).join("2026-10-17/a.JPG"),
            b"another file",
            Timestamp::from_unix_seconds(MODIFIED),
        );
    }
    let template = template();
    let settings = settings(&template);
    let clock = clock();

    let mut run_progress = Recorder::default();
    let summary = ImportRun::new(&by_run, &FnvHasher, &clock)
        .run(&settings, &mut run_progress)
        .unwrap();
    let mut step_progress = Recorder::default();
    let run = ImportRun::new(&by_steps, &FnvHasher, &clock);
    let plan = run.plan(&settings, &scan(&by_steps)).unwrap();
    let executed = run.execute(&settings, plan, &mut step_progress);

    assert_eq!(format!("{executed:?}"), format!("{summary:?}"));
    assert_eq!(step_progress.0, run_progress.0[1..]);
    assert_eq!(by_steps.paths(), by_run.paths());
}
