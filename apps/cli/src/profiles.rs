// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Finding and reading the import profile of `framehive import --profile <name>`
//! (CLI-010, ADR 0032, decision 3).

use std::ffi::OsStr;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use config::YamlParser;
use files::{EntryKind, FileSystem};
use import::ImportProfile;

/// The longest profile name (ADR 0032).
const MAX_NAME_LENGTH: usize = 64;

/// The file name extension of a profile.
const PROFILE_EXTENSION: &str = ".yaml";

/// The message when the configuration folder needs `HOME` and it is not absolute.
const UNKNOWN_CONFIGURATION_FOLDER: &str =
    "the configuration folder is unknown: set HOME to an absolute path";

/// The platform whose rule finds the configuration folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Platform {
    /// Linux, and every platform that is not macOS: the XDG base directories.
    Linux,
    /// macOS: `~/Library/Application Support`.
    MacOs,
}

impl Platform {
    /// The platform `framehive` was built for.
    pub(crate) fn current() -> Self {
        if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Linux
        }
    }
}

/// The configuration folder of CLI-010 for `platform`, `XDG_CONFIG_HOME` and `HOME`.
///
/// # Errors
///
/// Returns the message of CLI-010 when the folder needs `home` and it is not absolute.
pub(crate) fn configuration_folder(
    platform: Platform,
    xdg_config_home: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Result<PathBuf, String> {
    let home = home.map(Path::new).filter(|home| home.is_absolute());
    let base = match platform {
        Platform::Linux => match xdg_config_home.map(Path::new) {
            Some(config_home) if config_home.is_absolute() => Some(config_home.to_path_buf()),
            _ => home.map(|home| home.join(".config")),
        },
        Platform::MacOs => home.map(|home| home.join("Library/Application Support")),
    };
    base.map(|base| base.join("framehive"))
        .ok_or_else(|| UNKNOWN_CONFIGURATION_FOLDER.to_owned())
}

/// The configuration folder of this process, from its environment.
///
/// # Errors
///
/// Returns the message of CLI-010 when the folder is unknown.
pub(crate) fn process_configuration_folder() -> Result<PathBuf, String> {
    configuration_folder(
        Platform::current(),
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

/// The home folder that `~` in a profile means: `HOME`, when it is absolute.
///
/// # Errors
///
/// Returns a message when `HOME` is not set or not absolute.
pub(crate) fn process_home_folder() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .ok_or_else(|| "the home folder is unknown: set HOME to an absolute path".to_owned())
}

/// The profiles folder inside the configuration folder.
pub(crate) fn profiles_folder(configuration_folder: &Path) -> PathBuf {
    configuration_folder.join("profiles")
}

/// Checks a profile name against ADR 0032: 1 to 64 of `a`–`z`, `0`–`9`, `-` and `_`.
///
/// # Errors
///
/// Returns the message of CLI-010 for an invalid name.
pub(crate) fn check_profile_name(name: &str) -> Result<(), String> {
    if is_profile_name(name) {
        Ok(())
    } else {
        Err(format!(
            "invalid profile name {name}: use 1 to 64 of a-z, 0-9, - and _"
        ))
    }
}

fn is_profile_name(name: &str) -> bool {
    (1..=MAX_NAME_LENGTH).contains(&name.len())
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
}

/// Reads the profile `name` from the folder `profiles`, with `home` for `~`. The name
/// must have passed [`check_profile_name`].
///
/// # Errors
///
/// Returns the message of CLI-010 for a profile that is missing, cannot be read, is not
/// UTF-8 or is invalid.
pub(crate) fn read_profile(
    file_system: &dyn FileSystem,
    parser: &dyn YamlParser,
    profiles: &Path,
    name: &str,
    home: &Path,
) -> Result<ImportProfile, String> {
    let path = profile_path(profiles, name);
    let bytes = match read_bytes(file_system, &path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(missing_profile_message(file_system, profiles, name));
        }
        Err(error) => {
            return Err(format!(
                "the profile {} cannot be read: {error}",
                path.display()
            ));
        }
    };
    let text = String::from_utf8(bytes)
        .map_err(|_| format!("the profile {} is not valid UTF-8", path.display()))?;
    ImportProfile::read(parser, &path.display().to_string(), &text, home)
        .map_err(|error| error.to_string())
}

/// The file of the profile `name` in the folder `profiles`.
pub(crate) fn profile_path(profiles: &Path, name: &str) -> PathBuf {
    profiles.join(format!("{name}{PROFILE_EXTENSION}"))
}

fn read_bytes(file_system: &dyn FileSystem, path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file_system.open_read(path)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn missing_profile_message(file_system: &dyn FileSystem, profiles: &Path, name: &str) -> String {
    let known = known_profiles(file_system, profiles);
    let rest = if known.is_empty() {
        "there are no profiles yet, see framehive config init".to_owned()
    } else {
        format!("known profiles: {}", known.join(", "))
    };
    format!("no profile {name} in {}; {rest}", profiles.display())
}

/// The files `*.yaml` in `profiles` whose names are valid profile names, sorted,
/// without `.yaml`. A folder that cannot be listed has none.
fn known_profiles(file_system: &dyn FileSystem, profiles: &Path) -> Vec<String> {
    let mut known: Vec<String> = file_system
        .list_folder(profiles)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| matches!(entry.kind, EntryKind::File | EntryKind::SymbolicLink))
        .filter_map(|entry| {
            let name = entry
                .name
                .to_str()?
                .strip_suffix(PROFILE_EXTENSION)?
                .to_owned();
            is_profile_name(&name).then_some(name)
        })
        .collect();
    known.sort();
    known
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(
        platform: Platform,
        xdg_config_home: Option<&str>,
        home: Option<&str>,
    ) -> Result<PathBuf, String> {
        configuration_folder(
            platform,
            xdg_config_home.map(OsStr::new),
            home.map(OsStr::new),
        )
    }

    /// spec: CLI-010.12
    #[test]
    fn linux_uses_xdg_config_home_and_else_the_home_folder() {
        assert_eq!(
            folder(Platform::Linux, Some("/config"), Some("/home/ada")),
            Ok(PathBuf::from("/config/framehive"))
        );
        assert_eq!(
            folder(Platform::Linux, None, Some("/home/ada")),
            Ok(PathBuf::from("/home/ada/.config/framehive"))
        );
        assert_eq!(
            folder(Platform::Linux, Some("config"), Some("/home/ada")),
            Ok(PathBuf::from("/home/ada/.config/framehive"))
        );
        assert_eq!(
            folder(Platform::Linux, Some("/config"), None),
            Ok(PathBuf::from("/config/framehive"))
        );
    }

    /// spec: CLI-010.13
    #[test]
    fn macos_uses_application_support_and_ignores_xdg_config_home() {
        assert_eq!(
            folder(Platform::MacOs, Some("/config"), Some("/Users/ada")),
            Ok(PathBuf::from(
                "/Users/ada/Library/Application Support/framehive"
            ))
        );
    }

    /// spec: CLI-010.14
    #[test]
    fn a_folder_that_needs_a_home_folder_that_is_not_absolute_is_unknown() {
        for platform in [Platform::Linux, Platform::MacOs] {
            assert_eq!(
                folder(platform, None, Some("not-absolute")),
                Err(UNKNOWN_CONFIGURATION_FOLDER.to_owned())
            );
            assert_eq!(
                folder(platform, None, None),
                Err(UNKNOWN_CONFIGURATION_FOLDER.to_owned())
            );
        }
    }

    /// spec: CLI-010.9
    #[test]
    fn profile_names_follow_adr_0032() {
        let longest = "a".repeat(MAX_NAME_LENGTH);
        for name in ["holiday", "client-2026", "a_b", longest.as_str()] {
            assert!(is_profile_name(name), "{name}");
        }
        let too_long = "a".repeat(MAX_NAME_LENGTH + 1);
        for name in [
            "",
            "Holiday",
            "../holiday",
            "a.b",
            "a b",
            "ä",
            too_long.as_str(),
        ] {
            assert!(!is_profile_name(name), "{name}");
        }
    }
}
