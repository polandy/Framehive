// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The settings of an import run.

use std::path::{Path, PathBuf};

use naming::{CompatibilityMode, NameTemplate};

use crate::conflicts::ConflictPolicy;
use crate::job_code::{JobCode, require_job_code, require_video_job_code};
use crate::report::RunError;
use crate::sequence::MixedSequenceKinds;

/// What to import, from where to where, and how.
#[derive(Debug, Clone, Copy)]
pub struct ImportSettings<'a> {
    /// The folder to import from, absolute and without `..`.
    pub source: &'a Path,
    /// The import destination, absolute and without `..`.
    pub destination: &'a Path,
    /// The backup destinations, absolute and without `..`, in the order they are
    /// used (IMPORT-007). Each file goes to the same target path below each of them.
    pub backup_destinations: &'a [PathBuf],
    /// The video destination, absolute and without `..`: where videos are copied to
    /// (IMPORT-013). `None` copies them to the import destination.
    pub video_destination: Option<&'a Path>,
    /// The video backup destinations, absolute and without `..`, in the order they
    /// are used (IMPORT-013). Empty backs videos up to the backup destinations, so
    /// videos are never left without a backup while photos have one.
    pub video_backup_destinations: &'a [PathBuf],
    /// The name template for the target paths (IMPORT-001).
    pub template: &'a NameTemplate,
    /// The video name template: names the videos instead of `template` (IMPORT-018).
    pub video_template: Option<&'a NameTemplate>,
    /// The job code for `{job}` (IMPORT-006); required when a template uses it.
    pub job_code: Option<&'a JobCode>,
    /// Whether target names are kept compatible with every file system (IMPORT-001).
    pub mode: CompatibilityMode,
    /// What happens when a target name holds a different file (IMPORT-003).
    pub policy: ConflictPolicy,
    /// Plan and resolve conflicts, but write nothing (CLI-001, `--dry-run`).
    pub dry_run: bool,
}

impl ImportSettings<'_> {
    /// The name template, then the video name template if there is one.
    pub(crate) fn templates(&self) -> Vec<&NameTemplate> {
        std::iter::once(self.template)
            .chain(self.video_template)
            .collect()
    }

    /// Checks before a run starts that a template with `{job}` has a job code
    /// (IMPORT-006) and that both templates number by one kind (IMPORT-018).
    ///
    /// # Errors
    ///
    /// Returns [`RunError::MissingJobCode`] or [`RunError::MixedSequenceKinds`].
    pub(crate) fn check_templates(&self) -> Result<(), RunError> {
        require_job_code(self.template, self.job_code)?;
        if let Some(video_template) = self.video_template {
            require_video_job_code(video_template, self.job_code)?;
            MixedSequenceKinds::check(self.template, video_template)?;
        }
        Ok(())
    }
}
