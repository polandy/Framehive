// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The resources an import job names, and which import jobs therefore run in parallel
//! (IMPORT-015, "Resources", ADR 0036 §4). A job that asks before each file and gets no
//! answer keeps running until the test stops it, so nothing sleeps.

#![allow(clippy::unwrap_used, clippy::panic, reason = "tests")]

mod job_support;
mod run_support;

use std::path::{Path, PathBuf};

use files::FreeSpace;
use files::testing::MemoryFreeSpace;
use import::ImportJob;
use job_support::shared::{Shared, job_settings};
use job_support::{Steps, end_state, summary};
use jobs::{Job, JobId, JobQueue, JobState, JobUpdate, ResourceKey};

/// The source on file system 1, destinations on 2 and 3, the rest unknown.
fn free_space() -> MemoryFreeSpace {
    MemoryFreeSpace::unlimited()
        .with_file_system("/media/a", 1_000_000)
        .with_file_system("/media/b", 1_000_000)
        .with_file_system("/library", 1_000_000)
        .with_file_system("/other", 1_000_000)
}

/// The key of the file system `folder` lies on.
fn file_system(free_space: &MemoryFreeSpace, kind: &str, folder: &str) -> ResourceKey {
    let id = free_space
        .free_space(Path::new(folder))
        .unwrap()
        .file_system;
    ResourceKey::new(format!("{kind} file system {}", id.get()))
}

/// The update of `job` with `state`.
fn update(job: JobId, state: JobState) -> JobUpdate {
    JobUpdate { job, state }
}

/// spec: IMPORT-015.11
#[test]
fn an_import_job_names_its_source_and_destination_file_systems() {
    let free_space = MemoryFreeSpace::new()
        .with_file_system("/media/card", 1_000)
        .with_file_system("/library", 1_000)
        .with_file_system("/backup", 1_000);
    let shared = Shared::new(free_space.clone());
    let mut settings = job_settings("/media/card", "/library", "{name}.{ext}");
    settings.backup_destinations = vec![PathBuf::from("/backup")];
    settings.video_destination = Some(PathBuf::from("/videos"));

    let (job, _reports) = ImportJob::new(shared.ports(), settings);

    assert_eq!(
        job.resources(),
        [
            file_system(&free_space, "source", "/media/card"),
            file_system(&free_space, "destination", "/library"),
            ResourceKey::new("destination folder /videos"),
            file_system(&free_space, "destination", "/backup"),
        ]
    );
}

/// spec: IMPORT-015.12
#[test]
fn import_jobs_on_separate_devices_run_in_parallel() {
    let shared = Shared::new(free_space());
    shared.source("/media/a", &["a.jpg"]);
    shared.source("/media/b", &["b.jpg"]);
    let (waiting, _held) = shared.planned_job("/media/a", "/library", "{name}.{ext}");
    let (second, second_reports) = shared.planned_job("/media/b", "/other", "{name}.{ext}");
    let (mut queue, updates) = JobQueue::start();
    waiting
        .asking_before_each_file()
        .enqueue(&mut queue)
        .unwrap();

    let second = second.enqueue(&mut queue).unwrap();
    let copied = summary(second_reports.follow(&mut Steps::default()));

    assert_eq!(copied.files.len(), 1);
    assert!(
        shared
            .file_system
            .content(Path::new("/other/b.jpg"))
            .is_some()
    );
    assert!(
        shared
            .file_system
            .content(Path::new("/library/a.jpg"))
            .is_none()
    );
    assert_eq!(end_state(&updates, second), JobState::Finished);
}

/// spec: IMPORT-015.13
#[test]
fn import_jobs_on_one_destination_file_system_run_one_after_another() {
    let shared = Shared::new(free_space());
    shared.source("/media/a", &["a.jpg"]);
    shared.source("/media/b", &["b.jpg"]);
    let (first, _held) = shared.planned_job("/media/a", "/library", "{name}.{ext}");
    let (second, second_reports) = shared.planned_job("/media/b", "/library/other", "{name}.{ext}");
    let (mut queue, updates) = JobQueue::start();
    let first = first.asking_before_each_file().enqueue(&mut queue).unwrap();

    let second = second.enqueue(&mut queue).unwrap();
    queue.stop(first).unwrap();
    second_reports.follow(&mut Steps::default());
    queue.shut_down();

    let reported: Vec<JobUpdate> = updates.iter().collect();
    let at = |wanted: JobUpdate| {
        reported
            .iter()
            .position(|update| *update == wanted)
            .unwrap()
    };
    assert!(at(update(second, JobState::Queued)) < at(update(first, JobState::Stopped)));
    assert!(at(update(first, JobState::Stopped)) < at(update(second, JobState::Running)));
}
