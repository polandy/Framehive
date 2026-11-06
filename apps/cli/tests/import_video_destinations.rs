// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import --video-destination` and `--video-backup` (CLI-012), and the
//! video destination in the import log (CLI-009), against the real binary, the real
//! file system and a temporary folder. The child process runs with `TZ=UTC`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers may panic on bad fixtures"
)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use support::{DEFAULT_TARGET, Folders, has_line, lines, stderr, stdout};

const VIDEO_TARGET: &str = "2026/2026-10-17/clip.MOV";
const VIDEO_CONTENT: &[u8] = b"a video";

fn text(path: &Path) -> String {
    path.display().to_string()
}

/// An empty folder below the root.
fn empty_folder(folders: &Folders, name: &str) -> PathBuf {
    let folder = folders.root.path().join(name);
    fs::create_dir_all(&folder).unwrap();
    folder
}

/// The source of CLI-012.1: a RAW file and a video, both of 2026-10-17.
fn photo_and_video() -> Folders {
    let folders = Folders::new();
    folders.add_raf();
    folders.add("clip.MOV", VIDEO_CONTENT);
    folders
}

/// The files below `folder`, relative to it, sorted.
fn files_below(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    collect_files(folder, folder, &mut found);
    found.sort();
    found
}

fn collect_files(root: &Path, folder: &Path, found: &mut Vec<String>) {
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files(root, &path, found);
        } else {
            found.push(text(path.strip_prefix(root).unwrap()));
        }
    }
}

/// The line of the video, which ends with the mark of the video destination.
fn video_line(output: &Output) -> String {
    lines(output)
        .into_iter()
        .find(|line| line.starts_with("copied  clip.MOV -> "))
        .unwrap_or_else(|| panic!("{}", stdout(output)))
}

/// spec: CLI-012.1
#[test]
fn videos_go_to_the_video_destination() {
    let folders = photo_and_video();
    let videos = empty_folder(&folders, "videos");

    let output = folders.import(&["--video-destination", &text(&videos)]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(files_below(&folders.destination()), [DEFAULT_TARGET]);
    assert_eq!(files_below(&videos), [VIDEO_TARGET]);
    let line = video_line(&output);
    assert!(
        line.starts_with(&format!("copied  clip.MOV -> {VIDEO_TARGET}")),
        "{line}"
    );
    assert!(line.ends_with("  (video destination)"), "{line}");
    assert!(
        has_line(
            &output,
            &format!("copied  DSCF0001.RAF -> {DEFAULT_TARGET}")
        ),
        "{}",
        stdout(&output)
    );
}

/// spec: CLI-012.2
#[test]
fn videos_are_backed_up_to_the_video_backup_destinations() {
    let folders = photo_and_video();
    let videos = text(&empty_folder(&folders, "videos"));
    let backup = empty_folder(&folders, "backup");
    let video_backup = empty_folder(&folders, "video-backup");

    let output = folders.import(&[
        "--video-destination",
        &videos,
        "--backup",
        &text(&backup),
        "--video-backup",
        &text(&video_backup),
    ]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(files_below(&backup), [DEFAULT_TARGET]);
    assert_eq!(files_below(&video_backup), [VIDEO_TARGET]);
    let printed = lines(&output);
    let video = printed
        .iter()
        .position(|line| line.starts_with("copied  clip.MOV -> "))
        .unwrap();
    assert_eq!(
        printed[video + 1],
        format!("  backed up  {}", text(&video_backup))
    );
    let summary = printed
        .iter()
        .position(|line| line.ends_with(" left out"))
        .unwrap();
    assert_eq!(
        printed[summary + 1..summary + 3],
        [
            format!(
                "backup {}: 1 backed up, 0 already backed up, 0 failed",
                text(&backup)
            ),
            format!(
                "backup {}: 1 backed up, 0 already backed up, 0 failed",
                text(&video_backup)
            ),
        ]
    );
}

/// spec: CLI-012.3
#[test]
fn one_folder_backs_up_photos_and_videos() {
    let folders = photo_and_video();
    let videos = text(&empty_folder(&folders, "videos"));
    let backup = text(&empty_folder(&folders, "backup"));

    let output = folders.import(&[
        "--video-destination",
        &videos,
        "--backup",
        &backup,
        "--video-backup",
        &backup,
    ]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        files_below(Path::new(&backup)),
        [DEFAULT_TARGET, VIDEO_TARGET]
    );
    let backup_lines: Vec<String> = lines(&output)
        .into_iter()
        .filter(|line| line.starts_with("backup "))
        .collect();
    assert_eq!(
        backup_lines,
        [format!(
            "backup {backup}: 2 backed up, 0 already backed up, 0 failed"
        )]
    );
}

/// spec: CLI-012.4
#[test]
fn the_profile_supplies_the_video_settings_and_an_option_replaces_them() {
    let folders = photo_and_video();
    let videos = empty_folder(&folders, "holiday-videos");
    let video_backup = empty_folder(&folders, "holiday-video-backup");
    let other = empty_folder(&folders, "other-videos");
    folders.write_profile(
        "holiday",
        format!(
            "schema_version: 1\nvideo_destination: \"{}\"\nbackup:\n  video_destinations:\n    - \"{}\"\n",
            text(&videos),
            text(&video_backup)
        )
        .as_bytes(),
    );
    let source = text(&folders.source());
    let destination = text(&folders.destination());

    let output = folders.framehive(&["import", "--profile", "holiday", &source, &destination]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(files_below(&videos), [VIDEO_TARGET]);
    assert_eq!(files_below(&video_backup), [VIDEO_TARGET]);
    assert_eq!(files_below(&folders.destination()), [DEFAULT_TARGET]);

    folders.forget_history();
    let output = folders.framehive(&[
        "import",
        "--profile",
        "holiday",
        &source,
        &destination,
        "--video-destination",
        &text(&other),
    ]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(files_below(&other), [VIDEO_TARGET]);
}

/// spec: CLI-012.5
#[test]
fn an_invalid_video_destination_stops_the_command() {
    let folders = photo_and_video();
    let inside_source = text(&folders.source().join("videos"));
    let video_backup = text(&folders.root.path().join("video-backup"));
    let destination = text(&folders.destination());

    for (options, named) in [
        (vec!["--video-destination", &inside_source], &inside_source),
        (
            vec![
                "--video-backup",
                &video_backup,
                "--video-backup",
                &video_backup,
            ],
            &video_backup,
        ),
        (vec!["--video-backup", &destination], &destination),
    ] {
        let output = folders.import(&options);

        let error = stderr(&output);
        assert!(error.contains(named.as_str()), "{options:?}: {error}");
        assert!(stdout(&output).is_empty(), "{}", stdout(&output));
        assert!(!folders.destination().exists(), "{options:?}");
        assert!(!Path::new(&inside_source).exists(), "{options:?}");
        assert!(!Path::new(&video_backup).exists(), "{options:?}");
        assert_eq!(output.status.code(), Some(2), "{options:?}");
    }
}

/// spec: CLI-012.5
#[test]
fn a_backup_destination_that_is_the_video_destination_stops_the_command() {
    let folders = photo_and_video();
    let videos = text(&folders.root.path().join("videos"));

    let output = folders.import(&["--video-destination", &videos, "--backup", &videos]);

    let error = stderr(&output);
    assert_eq!(
        error,
        format!("error: invalid backup destination {videos}: it is the video destination\n")
    );
    assert!(!Path::new(&videos).exists());
    assert_eq!(output.status.code(), Some(2));
}

/// The format readiness when photos and videos require different numbers of locations:
/// the verdict names no number, which would be wrong for some of the files.
#[test]
fn the_verdict_names_no_number_when_files_require_different_numbers() {
    let folders = photo_and_video();
    let videos = text(&empty_folder(&folders, "videos"));
    let backup = text(&empty_folder(&folders, "backup"));
    let first = text(&empty_folder(&folders, "video-backup-1"));
    let second = text(&empty_folder(&folders, "video-backup-2"));

    let output = folders.import(&[
        "--video-destination",
        &videos,
        "--backup",
        &backup,
        "--video-backup",
        &first,
        "--video-backup",
        &second,
    ]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        lines(&output).last().unwrap(),
        "The source can be formatted: every file is verified in all its locations."
    );
}

/// spec: CLI-009.13
#[test]
fn the_import_log_shows_the_video_destination() {
    let folders = photo_and_video();
    let videos = text(&empty_folder(&folders, "videos"));
    let imported = folders.import(&["--video-destination", &videos]);
    assert_eq!(imported.status.code(), Some(0), "{}", stderr(&imported));

    let output = folders.import_log(&["1"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let printed = lines(&output);
    let import_destination = printed
        .iter()
        .position(|line| line.starts_with("import destination   "))
        .unwrap();
    assert_eq!(
        printed[import_destination + 1],
        format!("video destination    {videos}")
    );
    let block = printed.iter().position(|line| *line == videos).unwrap();
    assert!(
        printed[block + 1].starts_with(&format!("  clip.MOV -> {VIDEO_TARGET}  ")),
        "{}",
        stdout(&output)
    );

    let output = folders.import_log(&["1", "--format", "json"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let json = stdout(&output);
    assert!(
        json.contains(&format!("\"video_destination\": \"{videos}\"")),
        "{json}"
    );
}
