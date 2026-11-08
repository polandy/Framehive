// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading `config.yaml` in the configuration folder (CORE-006, IMPORT-019).

use std::io::{self, Read};
use std::path::Path;

use adapter_fs::StdFileSystem;
use adapter_yaml::SaphyrParser;
use config::YamlParser;
use files::FileSystem;
use import::{CONFIGURATION_FILE_NAME, Configuration};

use crate::profiles::process_configuration_folder;

/// Reads `config.yaml` in `configuration_folder`; a file that does not exist sets
/// nothing (IMPORT-019).
///
/// # Errors
///
/// Returns the message for a file that cannot be read, is not UTF-8 or is invalid.
pub(crate) fn read_configuration(
    file_system: &dyn FileSystem,
    parser: &dyn YamlParser,
    configuration_folder: &Path,
) -> Result<Configuration, String> {
    let path = configuration_folder.join(CONFIGURATION_FILE_NAME);
    let mut bytes = Vec::new();
    let read = file_system
        .open_read(&path)
        .and_then(|mut file| file.read_to_end(&mut bytes));
    match read {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(Configuration::default());
        }
        Err(error) => {
            return Err(format!("{} cannot be read: {error}", path.display()));
        }
    }
    let text =
        String::from_utf8(bytes).map_err(|_| format!("{} is not valid UTF-8", path.display()))?;
    Configuration::read(parser, &path.display().to_string(), &text)
        .map_err(|error| error.to_string())
}

/// Reads `config.yaml` of this process. Without a known configuration folder there is
/// no such file, so it sets nothing.
///
/// # Errors
///
/// Returns the message of [`read_configuration`].
pub(crate) fn process_configuration() -> Result<Configuration, String> {
    match process_configuration_folder() {
        Ok(folder) => read_configuration(&StdFileSystem::new(), &SaphyrParser::new(), &folder),
        Err(_) => Ok(Configuration::default()),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

    use domain::Timestamp;
    use files::testing::MemoryFileSystem;

    use super::*;

    const FOLDER: &str = "/home/ada/.config/framehive";

    fn read(file_system: &MemoryFileSystem) -> Result<Configuration, String> {
        read_configuration(file_system, &SaphyrParser::new(), Path::new(FOLDER))
    }

    #[test]
    fn a_missing_config_yaml_sets_nothing() {
        let file_system = MemoryFileSystem::new();
        file_system.add_folder(Path::new(FOLDER));

        assert_eq!(read(&file_system).unwrap(), Configuration::default());
    }

    #[test]
    fn config_yaml_is_read_from_the_configuration_folder() {
        let file_system = MemoryFileSystem::new();
        let text = b"schema_version: 1\nmetadata:\n  creator: [Andy Pollari]\n";
        let path = Path::new(FOLDER).join(CONFIGURATION_FILE_NAME);
        file_system.add_file(&path, text, Timestamp::from_unix_seconds(0));

        let creator = read(&file_system).unwrap().metadata.creator;

        assert_eq!(creator, Some(vec!["Andy Pollari".to_owned()]));
    }

    #[test]
    fn an_invalid_config_yaml_names_the_file() {
        let file_system = MemoryFileSystem::new();
        let path = Path::new(FOLDER).join(CONFIGURATION_FILE_NAME);
        file_system.add_file(
            &path,
            b"schema_version: 1\nauthor: Ada\n",
            Timestamp::from_unix_seconds(0),
        );

        let error = read(&file_system).unwrap_err();

        assert!(error.starts_with(&path.display().to_string()), "{error}");
    }
}
