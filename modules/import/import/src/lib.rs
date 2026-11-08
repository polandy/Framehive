// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `import` module of Framehive: business logic, use cases and ViewModels
//! (architecture §4.1). Its views live in `import-ui`. See `README.md`.

#![forbid(unsafe_code)]

mod backup;
mod configuration;
mod conflicts;
mod destinations;
mod dry_run;
mod free_space;
mod job;
mod job_code;
mod metadata_values;
mod plan;
mod profile;
mod profile_file_type;
mod readiness;
mod recent_job_codes;
mod report;
mod reserved_names;
mod roles;
mod run;
mod scan;
mod sequence;
mod view_model;

use kernel::{ModuleId, ModuleIdError};

pub use backup::{
    BackupCounts, BackupDestinationError, BackupDestinationProblem, BackupError, BackupOutcome,
    BackupReport, check_against_video_destination, check_backup_destinations,
};
pub use configuration::{
    CONFIGURATION, CONFIGURATION_FILE_NAME, CONFIGURATION_SCHEMA_ID, Configuration,
    configuration_json_schema,
};
pub use conflicts::{
    ConflictError, ConflictOutcome, ConflictPolicy, TargetRequest, TargetResolver,
};
pub use destinations::{
    DestinationNotCreated, VideoDestinationError, VideoDestinationProblem, check_video_destination,
};
pub use dry_run::DryRunMemory;
pub use free_space::{FreeSpaceCheck, FreeSpaceUnknown, NotEnoughSpace, SpaceShortage};
pub use job::{
    ImportJob, ImportJobEnd, ImportJobReports, ImportJobSettings, ImportPorts, OwnSource,
};
pub use job_code::{
    JobCode, JobCodeError, MAX_JOB_CODE_CHARACTERS, MissingJobCode, require_job_code,
    require_video_job_code,
};
pub use metadata_values::{ImportMetadata, Keyword, KeywordError, MetadataKeys};
pub use plan::{AssetFile, ImportPlan, PlannedFile};
pub use profile::ImportProfile;
pub use profile_file_type::{
    DEFAULT_NAME_TEMPLATE, IMPORT_PROFILE, IMPORT_PROFILE_SCHEMA_ID, IMPORT_PROFILE_V1,
    IMPORT_PROFILE_V1_SCHEMA_ID, import_profile_json_schema, import_profile_v1_json_schema,
};
pub use readiness::{CAMERA_FORMAT_READINESS_NOTE, FormatBlocker, FormatReadiness};
pub use recent_job_codes::{MAX_RECENT_JOB_CODES, RecentJobCodes};
pub use report::{
    CarriedSidecar, ExistingValue, ExistingValueResult, FileError, FileOutcome, FileReport,
    ImportSummary, NoProgress, ProgressSink, RunError, SidecarField,
};
pub use reserved_names::ReservedTargetNames;
pub use run::{FilePlan, ImportRun, ImportSettings, PlannedRun, RunFile};
pub use scan::{
    FoundFile, LeftOut, LeftOutReason, ScanError, SourceScan, SourceSidecar, scan_camera,
    scan_folder, scan_source,
};
pub use sequence::{MixedSequenceKinds, SessionSequence};
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
