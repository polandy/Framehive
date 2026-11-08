// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The settings of an import run.

use std::path::{Path, PathBuf};

use domain::AssetFileRole;
use metadata::xmp::SidecarNaming;
use naming::{CompatibilityMode, NameTemplate};

use crate::conflicts::ConflictPolicy;
use crate::job_code::{JobCode, require_job_code, require_video_job_code};
use crate::metadata_values::ImportMetadata;
use crate::report::RunError;
use crate::scan::{FoundFile, SourceSidecar};
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
    /// The values written to each photo's sidecar besides the job code (IMPORT-019);
    /// [`ImportMetadata::none`] writes none.
    pub metadata: &'a ImportMetadata,
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

    /// Whether the photos get sidecars: a job code or a `metadata` value is set
    /// (IMPORT-019.2).
    pub(crate) fn writes_sidecars(&self) -> bool {
        self.job_code.is_some() || self.metadata.has_values()
    }

    /// How the sidecar of a file with `role` is named, or `None` when it gets none:
    /// only photos do (IMPORT-019, "Which files get a sidecar").
    pub(crate) fn sidecar_naming_of(&self, role: AssetFileRole) -> Option<SidecarNaming> {
        let photo = matches!(role, AssetFileRole::Raw | AssetFileRole::Rendered);
        (photo && self.writes_sidecars()).then_some(self.metadata.sidecar_naming)
    }

    /// The sidecars `file` gets at its target: each source sidecar that came with it,
    /// in its own naming form, and the one with the values (IMPORT-019). The values go
    /// into the carried-along sidecar of the configured naming form, else into the
    /// first that came along, else into a new one (IMPORT-020).
    pub(crate) fn sidecars_of<'f>(&self, file: &'f FoundFile) -> Vec<PlannedSidecar<'f>> {
        let values = self.sidecar_naming_of(file.role);
        let holder = values.and_then(|naming| {
            file.sidecars
                .iter()
                .position(|sidecar| sidecar.naming == naming)
                .or((!file.sidecars.is_empty()).then_some(0))
        });
        let mut sidecars: Vec<PlannedSidecar<'f>> = file
            .sidecars
            .iter()
            .enumerate()
            .map(|(position, source)| PlannedSidecar {
                naming: source.naming,
                source: Some(source),
                holds_values: holder == Some(position),
            })
            .collect();
        if let (Some(naming), None) = (values, holder) {
            sidecars.push(PlannedSidecar {
                naming,
                source: None,
                holds_values: true,
            });
        }
        sidecars
    }

    /// The naming forms of the sidecars `file` gets; their names must be free with its
    /// target's (IMPORT-019, "Conflicts").
    pub(crate) fn sidecar_namings_of(&self, file: &FoundFile) -> Vec<SidecarNaming> {
        self.sidecars_of(file)
            .iter()
            .map(|sidecar| sidecar.naming)
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

/// One sidecar a file gets at its target.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlannedSidecar<'f> {
    /// How it is named.
    pub(crate) naming: SidecarNaming,
    /// The source sidecar it is made from; `None` for a new one.
    pub(crate) source: Option<&'f SourceSidecar>,
    /// Whether the import's values go into it.
    pub(crate) holds_values: bool,
}
