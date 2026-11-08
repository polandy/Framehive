// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive config init` (CLI-010): writes the example import profile.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, ExitCode};

use adapter_fs::StdFileSystem;
use files::FileSystem;

use crate::guided::create_folders;
use crate::profiles::{process_configuration_folder, profiles_folder};
use crate::{FILES_FAILED, NOT_STARTED, report_error};

/// The example profile: every key of IMPORT-011 with a comment that explains it and
/// names its default. `video_destination`, `naming.video_template` and
/// `backup.video_destinations` have no value of their own by default and are left as comments, so the file reads as the
/// defaults. The folder to import into is given on the command line, never here
/// (IMPORT-011).
const EXAMPLE_PROFILE: &str = r#"# yaml-language-server: $schema=https://polandy.github.io/framehive/schemas/import-profile-v1.json
#
# An import profile of Framehive. `framehive import --profile <name> <source> <destination>`
# uses it, where <name> is the file name without .yaml: 1 to 64 of a-z, 0-9, - and _.
# The folder to import into is always given on the command line, never here.
# Copy this file to start a profile. An option given on the command line replaces
# the value here, also for lists.

# Version of the format. Required.
schema_version: 1

# Folder to import videos into: an absolute path, ~, or a path that starts with ~/.
# Default: the destination given on the command line.
# video_destination: ~/Videos/Archive

naming:
  # Name template for the folders and name of each imported file.
  # Default: {date:%Y}/{date:%Y-%m-%d}/{name}.{ext}
  template: "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}"
  # Name template for each imported video; --video-template replaces it.
  # Default: the name template above.
  # video_template: "{date:%Y}/Videos/{name}.{ext}"
  # What to do when a target name holds a different file: add_suffix, skip or fail.
  # Default: add_suffix
  on_conflict: add_suffix

# Job codes that the {job} token of the template inserts; the first one is used,
# --job-code replaces it.
# Default: none, for example [Smith wedding, Jones].
job_codes: []

backup:
  # Folders to back up each imported file to, in this order.
  # Default: none, for example [/run/media/ada/Backup/Pictures].
  destinations: []
  # Folders to back up each imported video to, in this order. Leave the key out to
  # back up videos to the destinations above; an empty list is an error.
  # Default: the destinations above.
  # video_destinations: [/run/media/ada/Backup/Videos]
"#;

/// The file name of the example profile.
const EXAMPLE_FILE: &str = "example.yaml";

/// Runs `framehive config init` and returns the exit code of CLI-010.
pub(crate) fn run_config_init() -> ExitCode {
    let folder = match process_configuration_folder() {
        Ok(folder) => folder,
        Err(message) => {
            report_error(&message);
            return ExitCode::from(NOT_STARTED);
        }
    };
    let profiles = profiles_folder(&folder);
    match write_example(&StdFileSystem::new(), &profiles, process::id()) {
        Ok(path) => {
            // Nothing more can be reported when standard output is gone.
            let _ = writeln!(io::stdout().lock(), "wrote {}", path.display());
            ExitCode::SUCCESS
        }
        Err(message) => {
            report_error(&message);
            ExitCode::from(FILES_FAILED)
        }
    }
}

/// Writes the example profile into `profiles`, creating missing folders, and returns
/// its path. An existing file is never replaced: the file is written as
/// `<file>.<process_id>.part` and renamed without replacing. The process ID keeps
/// overlapping invocations apart: with one shared `.part` name, a second one would
/// truncate the file that the first one had just linked to the example's name.
///
/// # Errors
///
/// Returns the message of CLI-010 when the file exists or a folder or the file cannot
/// be written.
fn write_example(
    file_system: &dyn FileSystem,
    profiles: &Path,
    process_id: u32,
) -> Result<PathBuf, String> {
    let path = profiles.join(EXAMPLE_FILE);
    let exists = || format!("{} exists already; nothing was written", path.display());
    create_folders(file_system, profiles).map_err(|error| {
        format!(
            "the folder {} cannot be created: {error}",
            profiles.display()
        )
    })?;
    let cannot_write = |error: io::Error| format!("{} cannot be written: {error}", path.display());
    if file_system.entry(&path).map_err(cannot_write)?.is_some() {
        return Err(exists());
    }
    let part = part_path(&path, process_id);
    write_part(file_system, &part).map_err(cannot_write)?;
    match file_system.rename_no_replace(&part, &path) {
        Ok(()) => {}
        Err(error) => {
            // The `.part` file is ours; nothing more can be done when it stays.
            let _ = file_system.remove_file(&part);
            return Err(if error.kind() == io::ErrorKind::AlreadyExists {
                exists()
            } else {
                cannot_write(error)
            });
        }
    }
    file_system.sync_folder(profiles).map_err(cannot_write)?;
    Ok(path)
}

fn part_path(path: &Path, process_id: u32) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".{process_id}.part"));
    PathBuf::from(name)
}

fn write_part(file_system: &dyn FileSystem, part: &Path) -> io::Result<()> {
    let mut file = file_system.create_file(part)?;
    file.write_all(EXAMPLE_PROFILE.as_bytes())?;
    file.sync()
}

#[cfg(test)]
mod tests {
    use domain::Timestamp;
    use files::testing::MemoryFileSystem;
    use import::IMPORT_PROFILE_V1_SCHEMA_ID;

    use super::*;

    const PROFILES: &str = "/config/framehive/profiles";

    #[test]
    fn another_process_keeps_its_part_file() {
        let file_system = MemoryFileSystem::new();
        let example = Path::new(PROFILES).join(EXAMPLE_FILE);
        let other_part = part_path(&example, 7);
        file_system.add_folder(Path::new(PROFILES));
        file_system.add_file(&other_part, b"# other run", Timestamp::from_unix_seconds(0));

        let path = write_example(&file_system, Path::new(PROFILES), 8);

        assert_eq!(path, Ok(example.clone()));
        assert_eq!(
            file_system.content(&other_part),
            Some(b"# other run".to_vec())
        );
        assert_eq!(
            file_system.content(&example),
            Some(EXAMPLE_PROFILE.as_bytes().to_vec())
        );
        assert_eq!(file_system.content(&part_path(&example, 8)), None);
    }

    #[test]
    fn the_example_names_the_published_schema() {
        assert!(EXAMPLE_PROFILE.starts_with(&format!(
            "# yaml-language-server: $schema={IMPORT_PROFILE_V1_SCHEMA_ID}\n"
        )));
    }
}
