// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `import` module of Framehive: business logic, use cases and ViewModels
//! (architecture §4.1). Its views live in `import-ui`. See `README.md`.

#![forbid(unsafe_code)]

mod backup;
mod conflicts;
mod job_code;
mod plan;
mod readiness;
mod recent_job_codes;
mod report;
mod roles;
mod run;
mod scan;
mod sequence;
mod view_model;

use kernel::{ModuleId, ModuleIdError};

pub use backup::{
    BackupCounts, BackupDestinationError, BackupDestinationProblem, BackupError, BackupOutcome,
    BackupReport, check_backup_destinations,
};
pub use conflicts::{
    ConflictError, ConflictOutcome, ConflictPolicy, TargetRequest, TargetResolver,
};
pub use job_code::{
    JobCode, JobCodeError, MAX_JOB_CODE_CHARACTERS, MissingJobCode, require_job_code,
};
pub use plan::{AssetFile, ImportPlan, PlanNotice, PlannedFile};
pub use readiness::{FormatBlocker, FormatReadiness};
pub use recent_job_codes::{MAX_RECENT_JOB_CODES, RecentJobCodes};
pub use report::{
    FileError, FileOutcome, FileReport, ImportSummary, NoProgress, ProgressSink, RunError,
};
pub use run::{ImportRun, ImportSettings, PlannedRun, RunFile};
pub use scan::{
    FoundFile, LeftOut, LeftOutReason, ScanError, SourceScan, scan_folder, scan_source,
};
pub use sequence::SessionSequence;
pub use view_model::ExampleViewModel;

/// Stable, unique id of this module (architecture §4.2).
pub const MODULE_ID: &str = "import";

/// Returns [`MODULE_ID`] as a validated [`ModuleId`].
///
/// # Errors
///
/// Returns a [`ModuleIdError`] if [`MODULE_ID`] is not a valid module id; the
/// conformance test rules that out.
pub fn module_id() -> Result<ModuleId, ModuleIdError> {
    ModuleId::new(MODULE_ID)
}
