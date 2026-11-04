// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import --backup` (CLI-005) against the real binary, the real file system
//! and a temporary folder. The child process runs with `TZ=UTC`.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use metadata::testing::raf;
use support::{COPIED_LINE, DEFAULT_TARGET, Folders, has_line, lines, photo, stderr, stdout};

fn backup(folders: &Folders, name: &str) -> PathBuf {
    folders.root.path().join(name)
}

fn text(path: &Path) -> String {
    path.display().to_string()
}

/// spec: CLI-005.1
#[test]
fn imports_to_the_destination_and_the_backup_destination() {
    let folders = Folders::new();
    folders.add_raf();
    let backup = text(&backup(&folders, "backup"));

    let output = folders.import(&["--backup", &backup]);

    assert_eq!(
        stdout(&output),
        format!(
            "{COPIED_LINE}\n\
             \x20 backed up  {backup}\n\
             1 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n\
             backup {backup}: 1 backed up, 0 already backed up, 0 failed\n"
        )
    );
    for folder in [folders.destination(), PathBuf::from(&backup)] {
        assert_eq!(
            fs::read(folder.join(DEFAULT_TARGET)).unwrap(),
            raf(&photo())
        );
    }
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-005.2
#[test]
fn a_rerun_fills_a_new_backup_destination() {
    let folders = Folders::new();
    folders.add_raf();
    let backup = text(&backup(&folders, "backup"));
    let second = text(&self::backup(&folders, "second"));
    folders.import(&["--backup", &backup]);

    let output = folders.import(&["--backup", &backup, "--backup", &second]);

    let printed = lines(&output);
    let already = printed
        .iter()
        .position(|line| line == "already imported  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF")
        .unwrap_or_else(|| panic!("{}", stdout(&output)));
    assert_eq!(
        printed[already + 1..already + 3],
        [
            format!("  already backed up  {backup}"),
            format!("  backed up  {second}")
        ]
    );
    assert_eq!(
        fs::read(Path::new(&second).join(DEFAULT_TARGET)).unwrap(),
        raf(&photo())
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// spec: CLI-005.3
#[test]
fn a_backup_conflict_fails_with_exit_code_1() {
    let folders = Folders::new();
    folders.add_raf();
    let backup = backup(&folders, "backup");
    let occupied = backup.join(DEFAULT_TARGET);
    fs::create_dir_all(occupied.parent().unwrap()).unwrap();
    fs::write(&occupied, b"other content").unwrap();

    let output = folders.import(&["--backup", &text(&backup)]);

    assert_eq!(
        fs::read(folders.destination().join(DEFAULT_TARGET)).unwrap(),
        raf(&photo())
    );
    assert_eq!(fs::read(&occupied).unwrap(), b"other content");
    assert!(
        has_line(
            &output,
            &format!("  backup failed  {}: name conflict", text(&backup))
        ),
        "{}",
        stdout(&output)
    );
    assert_eq!(output.status.code(), Some(1));
}

/// spec: CLI-005.4
#[test]
fn an_invalid_backup_destination_stops_the_command() {
    let folders = Folders::new();
    folders.add_raf();
    let destination = text(&folders.destination());
    let inside_source = text(&folders.source().join("backup"));
    let backup = text(&backup(&folders, "backup"));

    for (options, named) in [
        (vec!["--backup", &destination], &destination),
        (vec!["--backup", &inside_source], &inside_source),
        (vec!["--backup", &backup, "--backup", &backup], &backup),
    ] {
        let output = folders.import(&options);

        let error = stderr(&output);
        assert!(error.contains(named.as_str()), "{error}");
        assert!(!folders.destination().exists(), "{options:?}");
        assert!(!folders.source().join("backup").exists(), "{options:?}");
        assert!(!Path::new(&backup).exists(), "{options:?}");
        assert_eq!(output.status.code(), Some(2), "{options:?}");
    }
}

#[test]
fn a_backup_destination_that_cannot_be_created_stops_the_command() {
    let folders = Folders::new();
    folders.add_raf();
    let file = folders.root.path().join("file");
    fs::write(&file, b"not a folder").unwrap();

    let output = folders.import(&["--backup", &text(&file.join("backup"))]);

    assert!(stderr(&output).contains("backup"), "{}", stderr(&output));
    assert!(
        !folders.destination().join(DEFAULT_TARGET).exists(),
        "{}",
        stdout(&output)
    );
    assert_eq!(output.status.code(), Some(2));
}

/// spec: CLI-005.5
#[test]
fn a_dry_run_writes_no_backup() {
    let folders = Folders::new();
    folders.add_raf();
    let backup = backup(&folders, "backup");

    let output = folders.import(&["--backup", &text(&backup), "--dry-run"]);

    assert_eq!(
        stdout(&output),
        "would copy  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF\n\
         1 would copy, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out\n"
    );
    assert!(!backup.exists());
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}
