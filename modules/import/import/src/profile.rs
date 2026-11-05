// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Import profiles: the settings of an import under a name, in a YAML file the user
//! edits (IMPORT-011, ADR 0032, decision 4).
//!
//! The `config` service does the strict reading of the file type; this module then
//! checks the values only the import understands (paths, the name template, job codes,
//! backup destinations) and reports them at the value, in the same form.

use std::path::{Path, PathBuf};

use config::{ConfigError, ConfigFile, DefaultValue, FileType, Key, Located, YamlParser};
use naming::NameTemplate;

use crate::conflicts::ConflictPolicy;
use crate::job_code::JobCode;

/// The name template of a profile that does not name one (IMPORT-011).
pub const DEFAULT_NAME_TEMPLATE: &str = "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}";

/// The `$id` of the published JSON Schema, `schemas/import-profile-v1.json`.
pub const IMPORT_PROFILE_SCHEMA_ID: &str =
    "https://polandy.github.io/framehive/schemas/import-profile-v1.json";

const ADD_SUFFIX: &str = "add_suffix";
const SKIP: &str = "skip";
const FAIL: &str = "fail";

const NAMING_KEYS: &[Key] = &[
    Key::text("template", "Name template (IMPORT-001)")
        .with_default(DefaultValue::Text(DEFAULT_NAME_TEMPLATE)),
    Key::choice(
        "on_conflict",
        "Conflict policy (IMPORT-003): add_suffix, skip or fail",
        &[ADD_SUFFIX, SKIP, FAIL],
    )
    .with_default(DefaultValue::Text(ADD_SUFFIX)),
];

const BACKUP_KEYS: &[Key] = &[Key::list_of_text(
    "destinations",
    "Backup destinations (IMPORT-007), in this order",
)];

/// The file type `import-profile`, version 1 (IMPORT-011, "Behaviour").
pub const IMPORT_PROFILE: FileType = FileType::new(
    "import-profile",
    1,
    &[
        Key::text("destination", "Folder to import into"),
        Key::mapping("naming", "How imported files are named", NAMING_KEYS),
        Key::list_of_text(
            "job_codes",
            "Job codes of the profile (IMPORT-006), the one to use first",
        ),
        Key::mapping("backup", "Backup copies of the import", BACKUP_KEYS),
    ],
);

/// The JSON Schema of [`IMPORT_PROFILE`], as published in
/// `schemas/import-profile-v1.json`.
#[must_use]
pub fn import_profile_json_schema() -> String {
    config::json_schema(&IMPORT_PROFILE, IMPORT_PROFILE_SCHEMA_ID)
}

/// The settings of one import profile, with every default applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportProfile {
    /// The folder to import into, if the profile names one.
    pub destination: Option<PathBuf>,
    /// The name template.
    pub template: NameTemplate,
    /// The conflict policy.
    pub on_conflict: ConflictPolicy,
    /// The job codes, trimmed, in file order; the first is the one to use.
    pub job_codes: Vec<JobCode>,
    /// The backup destinations, in file order.
    pub backup_destinations: Vec<PathBuf>,
}

impl ImportProfile {
    /// Reads the import profile `file` from `text`. `~` in a path means `home`.
    /// Reading is pure: `file` is used only in messages, and no path is checked
    /// against the file system.
    ///
    /// # Errors
    ///
    /// The first error of CORE-006, else the first invalid value of IMPORT-011, at
    /// the start of that value.
    pub fn read(
        parser: &dyn YamlParser,
        file: &str,
        text: &str,
        home: &Path,
    ) -> Result<Self, ConfigError> {
        let config = config::read(parser, &IMPORT_PROFILE, file, text)?;
        Self::from_config(&config, home)
    }

    /// Checks the values of a file that passed the strict reading of
    /// [`IMPORT_PROFILE`] and applies the defaults.
    ///
    /// # Errors
    ///
    /// The first invalid value, in the order of the keys of IMPORT-011.
    pub fn from_config(config: &ConfigFile, home: &Path) -> Result<Self, ConfigError> {
        let values = config.values();
        let destination = values
            .text("destination")
            .map(|path| profile_path(config, "destination", path, home))
            .transpose()?;
        let naming = values.mapping("naming");
        let template = match naming.and_then(|naming| naming.text("template")) {
            Some(template) => NameTemplate::parse(&template.value)
                .map_err(|error| config.error_at(template.position, error.kind.to_string()))?,
            None => NameTemplate::parse(DEFAULT_NAME_TEMPLATE)
                .map_err(|error| config.error_at(values.position(), error.to_string()))?,
        };
        let on_conflict = naming
            .and_then(|naming| naming.text("on_conflict"))
            .map_or(ConflictPolicy::AddSuffix, |policy| {
                conflict_policy(&policy.value)
            });
        let job_codes = job_codes(config, values.text_list("job_codes").unwrap_or_default())?;
        let backup_destinations = backup_destinations(
            config,
            values
                .mapping("backup")
                .and_then(|backup| backup.text_list("destinations"))
                .unwrap_or_default(),
            home,
        )?;
        Ok(ImportProfile {
            destination,
            template,
            on_conflict,
            job_codes,
            backup_destinations,
        })
    }
}

/// The policy of a choice the strict reading accepted.
fn conflict_policy(text: &str) -> ConflictPolicy {
    match text {
        SKIP => ConflictPolicy::Skip,
        FAIL => ConflictPolicy::Fail,
        _ => ConflictPolicy::AddSuffix,
    }
}

fn job_codes(
    config: &ConfigFile,
    entries: &[Located<String>],
) -> Result<Vec<JobCode>, ConfigError> {
    let mut codes: Vec<JobCode> = Vec::with_capacity(entries.len());
    for entry in entries {
        let code = JobCode::parse(&entry.value)
            .map_err(|error| config.error_at(entry.position, error.to_string()))?;
        if codes.contains(&code) {
            return Err(config.error_at(entry.position, format!("repeated job code {code}")));
        }
        codes.push(code);
    }
    Ok(codes)
}

fn backup_destinations(
    config: &ConfigFile,
    entries: &[Located<String>],
    home: &Path,
) -> Result<Vec<PathBuf>, ConfigError> {
    let mut paths: Vec<PathBuf> = Vec::with_capacity(entries.len());
    for entry in entries {
        let path = profile_path(config, "backup.destinations", entry, home)?;
        if paths.contains(&path) {
            return Err(config.error_at(
                entry.position,
                format!("repeated backup destination {}", entry.value),
            ));
        }
        paths.push(path);
    }
    Ok(paths)
}

/// The path under `key`, with `~` replaced by `home`, or an error at the value.
fn profile_path(
    config: &ConfigFile,
    key: &str,
    path: &Located<String>,
    home: &Path,
) -> Result<PathBuf, ConfigError> {
    expand_path(&path.value, home).ok_or_else(|| {
        config.error_at(
            path.position,
            format!("{key} must be an absolute path or start with ~/"),
        )
    })
}

/// `text` as a path: absolute as written, or `~` or `~/…` below `home`; `None` for
/// any other text.
fn expand_path(text: &str, home: &Path) -> Option<PathBuf> {
    if text == "~" {
        return Some(home.to_path_buf());
    }
    if let Some(below_home) = text.strip_prefix("~/") {
        let below_home = below_home.trim_start_matches('/');
        return Some(if below_home.is_empty() {
            home.to_path_buf()
        } else {
            home.join(below_home)
        });
    }
    let path = Path::new(text);
    path.is_absolute().then(|| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/home/ada";

    fn expanded(text: &str) -> Option<String> {
        expand_path(text, Path::new(HOME)).map(|path| path.display().to_string())
    }

    #[test]
    fn expands_the_home_folder_and_keeps_absolute_paths() {
        assert_eq!(expanded("~").as_deref(), Some(HOME));
        assert_eq!(expanded("~/").as_deref(), Some(HOME));
        assert_eq!(
            expanded("~/Pictures").as_deref(),
            Some("/home/ada/Pictures")
        );
        assert_eq!(expanded("/mnt/backup").as_deref(), Some("/mnt/backup"));
    }

    #[test]
    fn rejects_relative_paths_and_other_users_homes() {
        for text in ["Pictures", "../Backup", "~bob/Pictures", "~Pictures", ""] {
            assert_eq!(expanded(text), None, "{text}");
        }
    }

    #[test]
    fn the_default_template_is_valid() {
        assert!(NameTemplate::parse(DEFAULT_NAME_TEMPLATE).is_ok());
    }
}
