// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The file type `import-profile`: its keys, versions and JSON Schemas (IMPORT-011,
//! IMPORT-019).

use config::{DefaultValue, FileType, Key};

use crate::metadata_values::METADATA_KEY;

/// The name template of a profile that does not name one (IMPORT-011).
pub const DEFAULT_NAME_TEMPLATE: &str = "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}";

/// The `$id` of the published JSON Schema, `schemas/import-profile-v2.json`.
pub const IMPORT_PROFILE_SCHEMA_ID: &str =
    "https://polandy.github.io/framehive/schemas/import-profile-v2.json";

/// The `$id` of the JSON Schema of version 1, `schemas/import-profile-v1.json`, which
/// stays published for the profiles that name it.
pub const IMPORT_PROFILE_V1_SCHEMA_ID: &str =
    "https://polandy.github.io/framehive/schemas/import-profile-v1.json";

/// The version that brought the key `metadata` (IMPORT-019).
pub(crate) const METADATA_VERSION: u32 = 2;

const ADD_SUFFIX: &str = "add_suffix";
pub(crate) const SKIP: &str = "skip";
pub(crate) const FAIL: &str = "fail";

const NAMING_KEYS: &[Key] = &[
    Key::text("template", "Name template (IMPORT-001)")
        .with_default(DefaultValue::Text(DEFAULT_NAME_TEMPLATE)),
    Key::text(
        "video_template",
        "Name template for videos (IMPORT-018); left out, videos use the name template",
    ),
    Key::choice(
        "on_conflict",
        "Conflict policy (IMPORT-003): add_suffix, skip or fail",
        &[ADD_SUFFIX, SKIP, FAIL],
    )
    .with_default(DefaultValue::Text(ADD_SUFFIX)),
];

const BACKUP_KEYS: &[Key] = &[
    Key::list_of_text(
        "destinations",
        "Backup destinations (IMPORT-007), in this order",
    ),
    Key::list_of_text(
        "video_destinations",
        "Backup destinations for videos (IMPORT-013), in this order; \
         left out, videos go to the backup destinations",
    ),
];

const VIDEO_DESTINATION_KEY: Key = Key::text(
    "video_destination",
    "Folder to import videos into (IMPORT-013); left out, videos go to the destination given on the command line",
);
const NAMING_KEY: Key = Key::mapping("naming", "How imported files are named", NAMING_KEYS);
const JOB_CODES_KEY: Key = Key::list_of_text(
    "job_codes",
    "Job codes of the profile (IMPORT-006), the one to use first",
);
const BACKUP_KEY: Key = Key::mapping("backup", "Backup copies of the import", BACKUP_KEYS);

/// The file type `import-profile`, version 2 (IMPORT-011, "Behaviour"); a version 1
/// file is read too, without the key `metadata`.
pub const IMPORT_PROFILE: FileType = FileType::new(
    "import-profile",
    METADATA_VERSION,
    &[
        VIDEO_DESTINATION_KEY,
        NAMING_KEY,
        JOB_CODES_KEY,
        BACKUP_KEY,
        METADATA_KEY,
    ],
);

/// The file type `import-profile` as version 1 described it, for its JSON Schema.
pub const IMPORT_PROFILE_V1: FileType = FileType::new(
    "import-profile",
    1,
    &[VIDEO_DESTINATION_KEY, NAMING_KEY, JOB_CODES_KEY, BACKUP_KEY],
);

/// The JSON Schema of [`IMPORT_PROFILE`], as published in
/// `schemas/import-profile-v2.json`.
#[must_use]
pub fn import_profile_json_schema() -> String {
    config::json_schema(&IMPORT_PROFILE, IMPORT_PROFILE_SCHEMA_ID)
}

/// The JSON Schema of [`IMPORT_PROFILE_V1`], as published in
/// `schemas/import-profile-v1.json`.
#[must_use]
pub fn import_profile_v1_json_schema() -> String {
    config::json_schema(&IMPORT_PROFILE_V1, IMPORT_PROFILE_V1_SCHEMA_ID)
}
