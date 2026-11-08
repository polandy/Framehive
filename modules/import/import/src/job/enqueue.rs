// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What an import job reserves when it is enqueued, and the resources it names
//! (IMPORT-015, "Enqueueing", "Resources"; ADR 0036 §4–§6).

use std::path::{Path, PathBuf};

use domain::AssetFileRole;
use jobs::{JobId, JobQueue, ResourceKey};

use super::ImportJob;
use crate::conflicts::sidecar_target;
use crate::report::RunError;
use crate::run::{FilePlan, ImportSettings, PlannedRun};

impl ImportJob {
    /// Enqueues the job in `queue`. A job given its plan first reserves the sequence
    /// range of the plan and the target names of its planned files, unless it is a
    /// dry run without the dry-run memory (IMPORT-016); a job without a plan reserves
    /// its numbers when it starts.
    ///
    /// # Errors
    ///
    /// Returns [`RunError::Sequence`] if the numbers cannot be reserved; then the job
    /// is not enqueued and nothing is reserved (IMPORT-015.15).
    pub fn enqueue(mut self, queue: &mut JobQueue) -> Result<JobId, RunError> {
        if let Some(plan) = self.plan.take() {
            let settings = self.settings.settings();
            let reserves = !settings.dry_run || self.dry_run_memory.is_some();
            let plan = if reserves {
                self.ports.import_run().reserve_numbers(&settings, plan)?
            } else {
                plan
            };
            if reserves {
                let targets = planned_targets(&plan, &settings);
                self.reserved = Some(
                    self.ports
                        .reserved_names
                        .reserve(targets.iter().map(PathBuf::as_path)),
                );
            }
            self.plan = Some(plan);
        }
        Ok(queue.enqueue(Box::new(self)))
    }

    /// The source device and the file systems of the destinations (IMPORT-015.11).
    pub(super) fn resource_keys(&self) -> Vec<ResourceKey> {
        let settings = self.settings.settings();
        let source = match &self.source {
            Some((_, name)) => format!("source camera {}", name.display()),
            None => self.file_system_key("source", settings.source),
        };
        let mut keys = vec![source];
        let destinations = [settings.destination]
            .into_iter()
            .chain(settings.video_destination)
            .chain(settings.all_backup_destinations());
        for destination in destinations {
            let key = self.file_system_key("destination", destination);
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        keys.into_iter().map(ResourceKey::new).collect()
    }

    /// The file system `folder` lies on, or the folder itself when the free space port
    /// cannot tell it, so that it is never taken for another file system.
    fn file_system_key(&self, kind: &str, folder: &Path) -> String {
        match self.ports.free_space.free_space(folder) {
            Ok(space) => format!("{kind} file system {}", space.file_system.get()),
            Err(_) => format!("{kind} folder {}", folder.display()),
        }
    }
}

/// The absolute targets of the files `plan` names, and of the sidecars they get
/// (IMPORT-019): at the video destination for a video when it is a folder of its
/// own, else at the import destination.
fn planned_targets(plan: &PlannedRun, settings: &ImportSettings<'_>) -> Vec<PathBuf> {
    plan.files()
        .iter()
        .filter_map(|file| match &file.planned {
            Ok(FilePlan::Named(planned)) => {
                let target = planned.target.as_ref().ok()?;
                let destination = match (file.found.role, settings.separate_video_destination()) {
                    (AssetFileRole::Video, Some(video_destination)) => video_destination,
                    _ => settings.destination,
                };
                let sidecar = settings
                    .sidecar_naming_of(file.found.role)
                    .map(|naming| destination.join(sidecar_target(naming, target)));
                Some([Some(destination.join(target)), sidecar])
            }
            _ => None,
        })
        .flatten()
        .flatten()
        .collect()
}
