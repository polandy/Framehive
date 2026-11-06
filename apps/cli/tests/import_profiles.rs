// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Import profiles (IMPORT-011) end to end: `import::ImportProfile::read` with the
//! real YAML parser. They live here because the import module must not depend on an
//! adapter, not even in its tests (architecture §3); the app is the lowest layer that
//! knows both.

#![allow(clippy::unwrap_used, reason = "tests may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use adapter_yaml::SaphyrParser;
use import::{
    ConflictPolicy, DEFAULT_NAME_TEMPLATE, IMPORT_PROFILE_SCHEMA_ID, ImportProfile, JobCode,
};
use support::{Folders, stderr};

const FILE: &str = "wedding.yaml";
const HOME: &str = "/home/ada";

fn read(text: &str) -> ImportProfile {
    ImportProfile::read(&SaphyrParser::new(), FILE, text, Path::new(HOME)).unwrap()
}

/// The error as `<line>:<column>: <message>`, without the file name.
fn located_message(text: &str) -> String {
    let error = ImportProfile::read(&SaphyrParser::new(), FILE, text, Path::new(HOME)).unwrap_err();
    assert_eq!(error.file, FILE);
    format!("{}: {}", error.position, error.problem)
}

fn job_codes(profile: &ImportProfile) -> Vec<&str> {
    profile.job_codes.iter().map(JobCode::as_str).collect()
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every fenced YAML block in the Markdown files under `folder` that declares the
/// schema of the import profile, as `(where, text)`.
fn profiles_in_markdown(folder: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut entries: Vec<PathBuf> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            found.extend(profiles_in_markdown(&path));
        } else if path.extension().is_some_and(|extension| extension == "md") {
            let text = fs::read_to_string(&path).unwrap();
            for block in yaml_blocks(&text) {
                if block.contains(IMPORT_PROFILE_SCHEMA_ID) {
                    found.push((path.display().to_string(), block));
                }
            }
        }
    }
    found
}

fn yaml_blocks(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in markdown.lines() {
        match current.as_mut() {
            None if line.trim() == "```yaml" => current = Some(String::new()),
            None => {}
            Some(_) if line.trim() == "```" => blocks.extend(current.take()),
            Some(block) => {
                block.push_str(line);
                block.push('\n');
            }
        }
    }
    blocks
}

/// The example profile that `framehive config init` writes (CLI-010).
fn config_init_example() -> (String, String) {
    let folders = Folders::new();
    let output = folders.framehive(&["config", "init"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let path = folders.profiles_folder().join("example.yaml");
    (
        "framehive config init".to_owned(),
        fs::read_to_string(path).unwrap(),
    )
}

/// Every import profile the project shows its users.
fn shown_profiles() -> Vec<(String, String)> {
    let root = repository_root();
    let mut profiles = profiles_in_markdown(&root.join("docs"));
    profiles.extend(profiles_in_markdown(&root.join("specs")));
    profiles.push(config_init_example());
    profiles
}

fn adr_0032_example() -> String {
    shown_profiles()
        .into_iter()
        .find(|(path, _)| path.contains("0032-import-profiles"))
        .map(|(_, text)| text)
        .unwrap()
}

/// spec: IMPORT-011.1
#[test]
fn a_minimal_profile_takes_the_defaults() {
    let profile = read("schema_version: 1\n");
    assert_eq!(profile.destination, None);
    assert_eq!(profile.template.as_str(), DEFAULT_NAME_TEMPLATE);
    assert_eq!(
        DEFAULT_NAME_TEMPLATE,
        "{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}"
    );
    assert_eq!(profile.on_conflict, ConflictPolicy::AddSuffix);
    assert!(profile.job_codes.is_empty());
    assert!(profile.backup_destinations.is_empty());
}

/// spec: IMPORT-011.2
#[test]
fn a_full_profile() {
    let profile = read(&adr_0032_example());
    assert_eq!(
        profile.destination,
        Some(PathBuf::from("/home/ada/Pictures/Archive"))
    );
    assert_eq!(
        profile.template.as_str(),
        "{date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}"
    );
    assert_eq!(profile.on_conflict, ConflictPolicy::AddSuffix);
    assert_eq!(job_codes(&profile), ["Smith wedding"]);
    assert_eq!(
        profile.backup_destinations,
        [PathBuf::from("/run/media/ada/Backup/Pictures")]
    );
}

/// spec: IMPORT-011.3
#[test]
fn relative_paths_are_rejected() {
    assert_eq!(
        located_message("schema_version: 1\ndestination: Pictures\n"),
        "2:14: destination must be an absolute path or start with ~/"
    );
    assert_eq!(
        located_message(
            "schema_version: 1\nbackup:\n  destinations:\n    - /mnt/backup\n    - ../Backup\n"
        ),
        "5:7: backup.destinations must be an absolute path or start with ~/"
    );
    assert_eq!(
        located_message("schema_version: 1\ndestination: ~Pictures\n"),
        "2:14: destination must be an absolute path or start with ~/"
    );
}

/// spec: IMPORT-011.4
#[test]
fn a_tilde_alone_is_the_home_folder() {
    let profile = read("schema_version: 1\ndestination: \"~\"\n");
    assert_eq!(profile.destination, Some(PathBuf::from(HOME)));
    let profile = read("schema_version: 1\ndestination: ~/\n");
    assert_eq!(profile.destination, Some(PathBuf::from(HOME)));
}

/// spec: IMPORT-011.5
#[test]
fn an_invalid_template_is_an_error_at_the_value() {
    assert_eq!(
        located_message("schema_version: 1\nnaming:\n  template: \"{date:%Y}/{unknown}.{ext}\"\n"),
        "3:13: unknown token {unknown}"
    );
}

/// spec: IMPORT-011.6
#[test]
fn an_unknown_conflict_policy_is_an_error_at_the_value() {
    assert_eq!(
        located_message("schema_version: 1\nnaming:\n  on_conflict: overwrite\n"),
        "3:16: expected add_suffix, skip or fail, found overwrite"
    );
    let profile = read("schema_version: 1\nnaming:\n  on_conflict: fail\n");
    assert_eq!(profile.on_conflict, ConflictPolicy::Fail);
}

/// spec: IMPORT-011.7
#[test]
fn job_codes_are_checked_and_trimmed() {
    let profile = read("schema_version: 1\njob_codes: [\" Smith wedding \", Jones]\n");
    assert_eq!(job_codes(&profile), ["Smith wedding", "Jones"]);
    assert_eq!(
        located_message("schema_version: 1\njob_codes: [\"a/b\"]\n"),
        "2:13: a job code must not contain \"/\""
    );
    assert_eq!(
        located_message("schema_version: 1\njob_codes: [Smith, \" Smith\"]\n"),
        "2:20: repeated job code Smith"
    );
}

/// spec: IMPORT-011.8
#[test]
fn a_repeated_backup_destination_is_an_error_at_the_second() {
    assert_eq!(
        located_message("schema_version: 1\nbackup:\n  destinations: [/mnt/backup, /mnt/backup]\n"),
        "3:31: repeated backup destination /mnt/backup"
    );
}

/// spec: IMPORT-011.9
#[test]
fn an_unknown_key_inside_a_section_gets_a_suggestion() {
    assert_eq!(
        located_message("schema_version: 1\nnaming:\n  templte: \"{name}.{ext}\"\n"),
        "3:3: unknown key templte; did you mean template?"
    );
}

/// spec: IMPORT-011.11
#[test]
fn every_shown_profile_is_valid() {
    let profiles = shown_profiles();
    assert!(
        !profiles.is_empty(),
        "no import profile found in docs/ or specs/"
    );
    for (path, text) in profiles {
        let result = ImportProfile::read(&SaphyrParser::new(), &path, &text, Path::new(HOME));
        assert!(result.is_ok(), "{path}: {}", result.unwrap_err());
    }
}

/// spec: IMPORT-011.12
#[test]
fn video_settings_are_read_and_an_empty_video_backup_list_is_an_error() {
    let profile = read(
        "schema_version: 1\nvideo_destination: ~/Videos\nbackup:\n  video_destinations: [/mnt/backup/Videos]\n",
    );
    assert_eq!(
        profile.video_destination,
        Some(PathBuf::from("/home/ada/Videos"))
    );
    assert_eq!(
        profile.video_backup_destinations,
        [PathBuf::from("/mnt/backup/Videos")]
    );
    let profile = read("schema_version: 1\n");
    assert_eq!(profile.video_destination, None);
    assert!(profile.video_backup_destinations.is_empty());
    assert_eq!(
        located_message("schema_version: 1\nbackup:\n  video_destinations: []\n"),
        "3:23: leave backup.video_destinations out to back up videos to the backup destinations"
    );
    assert_eq!(
        located_message(
            "schema_version: 1\nbackup:\n  video_destinations: [/mnt/videos, /mnt/videos]\n"
        ),
        "3:37: repeated video backup destination /mnt/videos"
    );
    assert_eq!(
        located_message("schema_version: 1\nvideo_destination: Videos\n"),
        "2:20: video_destination must be an absolute path or start with ~/"
    );
}
