// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Import profiles: the settings of an import under a name, in a YAML file the user
//! edits (IMPORT-011, ADR 0032, decision 4).
//!
//! The `config` service does the strict reading of the file type; this module then
//! checks the values only the import understands (paths, the name template, job codes,
//! backup destinations) and reports them at the value, in the same form.

use std::path::{Path, PathBuf};

use config::{
    ConfigError, ConfigFile, Located, Position, Problem, Values, YamlNode, YamlParser, YamlValue,
};
use naming::NameTemplate;

use crate::conflicts::ConflictPolicy;
use crate::job_code::JobCode;
use crate::metadata_values::{METADATA_KEY, MetadataKeys};
use crate::profile_file_type::{
    DEFAULT_NAME_TEMPLATE, FAIL, IMPORT_PROFILE, METADATA_VERSION, SKIP,
};

/// The settings of one import profile, with every default applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportProfile {
    /// The name template.
    pub template: NameTemplate,
    /// The name template for videos, if the profile names one (IMPORT-018).
    pub video_template: Option<NameTemplate>,
    /// The conflict policy.
    pub on_conflict: ConflictPolicy,
    /// The job codes, trimmed, in file order; the first is the one to use.
    pub job_codes: Vec<JobCode>,
    /// The backup destinations, in file order.
    pub backup_destinations: Vec<PathBuf>,
    /// The folder to import videos into, if the profile names one (IMPORT-013).
    pub video_destination: Option<PathBuf>,
    /// The backup destinations for videos, in file order. Empty when the profile
    /// leaves them out, which means the backup destinations: an empty list in the
    /// file is an error, so empty never means "no video backup".
    pub video_backup_destinations: Vec<PathBuf>,
    /// The `metadata` keys the profile sets (IMPORT-019); each replaces the same key
    /// of `config.yaml`.
    pub metadata: MetadataKeys,
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
        let config = config::read(parser, &IMPORT_PROFILE, file, text)
            .map_err(|error| explain_destination(parser, text, error))?;
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
        let video_destination = values
            .text("video_destination")
            .map(|path| profile_path(config, "video_destination", path, home))
            .transpose()?;
        let naming = values.mapping("naming");
        let template = match naming.and_then(|naming| naming.text("template")) {
            Some(template) => NameTemplate::parse(&template.value)
                .map_err(|error| config.error_at(template.position, error.kind.to_string()))?,
            None => NameTemplate::parse(DEFAULT_NAME_TEMPLATE)
                .map_err(|error| config.error_at(values.position(), error.to_string()))?,
        };
        let video_template = naming
            .and_then(|naming| naming.text("video_template"))
            .map(|template| {
                NameTemplate::parse(&template.value)
                    .map_err(|error| config.error_at(template.position, error.kind.to_string()))
            })
            .transpose()?;
        let on_conflict = naming
            .and_then(|naming| naming.text("on_conflict"))
            .map_or(ConflictPolicy::AddSuffix, |policy| {
                conflict_policy(&policy.value)
            });
        let job_codes = job_codes(config, values.text_list("job_codes").unwrap_or_default())?;
        let backup = values.mapping("backup");
        let backup_destinations = backup_destinations(
            config,
            backup
                .and_then(|backup| backup.text_list("destinations"))
                .unwrap_or_default(),
            home,
        )?;
        let video_backup_destinations = match backup {
            Some(backup) => video_backup_destinations(config, backup, home)?,
            None => Vec::new(),
        };
        let metadata = profile_metadata(config)?;
        Ok(ImportProfile {
            template,
            video_template,
            on_conflict,
            job_codes,
            backup_destinations,
            video_destination,
            video_backup_destinations,
            metadata,
        })
    }
}

/// The `metadata` keys, which a version 1 profile does not know.
fn profile_metadata(config: &ConfigFile) -> Result<MetadataKeys, ConfigError> {
    let values = config.values();
    match values.position_of(METADATA_KEY.name()) {
        Some(position) if config.schema_version() < METADATA_VERSION => Err(config.error_at(
            position,
            format!("metadata needs schema_version {METADATA_VERSION}"),
        )),
        _ => MetadataKeys::from_config(config),
    }
}

/// The key a profile once had for the folder to import into (ADR 0036, decision 8).
const REMOVED_DESTINATION: &str = "destination";

/// Why a profile has no `destination`, which IMPORT-011 gives instead of the message
/// for an unknown key.
const DESTINATION_NOT_READ: &str =
    "destination is not read from a profile; give the folder to import into on the command line";

/// Replaces the unknown-key error of a top-level `destination` with the reason the
/// key is gone, so that someone who wrote it learns where the folder goes now
/// (IMPORT-011, "Behaviour"). Every other error stays as it is.
fn explain_destination(parser: &dyn YamlParser, text: &str, error: ConfigError) -> ConfigError {
    let is_unknown_destination = matches!(
        &error.problem,
        Problem::UnknownKey { key, .. } if key == REMOVED_DESTINATION
    );
    if is_unknown_destination && is_top_level_key(parser, text, error.position) {
        return ConfigError {
            problem: Problem::Invalid(DESTINATION_NOT_READ.to_owned()),
            ..error
        };
    }
    error
}

/// Whether a key of the top-level mapping of `text` starts at `position`; a key of
/// the same name inside `naming` or `backup` is an ordinary unknown key.
fn is_top_level_key(parser: &dyn YamlParser, text: &str, position: Position) -> bool {
    let Ok(Some(YamlNode {
        value: YamlValue::Mapping(entries),
        ..
    })) = parser.parse(text)
    else {
        return false;
    };
    entries.iter().any(|entry| entry.key.position == position)
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
    distinct_paths(
        config,
        entries,
        home,
        "backup.destinations",
        "repeated backup destination",
    )
}

/// `backup.video_destinations`: like the backup destinations, but an empty list is
/// an error, because videos without a backup are not offered (IMPORT-013).
fn video_backup_destinations(
    config: &ConfigFile,
    backup: &Values,
    home: &Path,
) -> Result<Vec<PathBuf>, ConfigError> {
    let Some(entries) = backup.text_list("video_destinations") else {
        return Ok(Vec::new());
    };
    if entries.is_empty() {
        let position = backup
            .position_of("video_destinations")
            .unwrap_or_else(|| backup.position());
        return Err(config.error_at(
            position,
            "leave backup.video_destinations out to back up videos to the backup destinations",
        ));
    }
    distinct_paths(
        config,
        entries,
        home,
        "backup.video_destinations",
        "repeated video backup destination",
    )
}

/// The paths of a list under `key`, each checked; an entry equal to an earlier one
/// is an error `<repeated> <path>` at that entry.
fn distinct_paths(
    config: &ConfigFile,
    entries: &[Located<String>],
    home: &Path,
    key: &str,
    repeated: &str,
) -> Result<Vec<PathBuf>, ConfigError> {
    let mut paths: Vec<PathBuf> = Vec::with_capacity(entries.len());
    for entry in entries {
        let path = profile_path(config, key, entry, home)?;
        if paths.contains(&path) {
            return Err(config.error_at(entry.position, format!("{repeated} {}", entry.value)));
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
