// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` with a free space in memory (CLI-011): the process tests cannot
//! replace the real one.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use files::testing::{MemoryFreeSpace, TempDir};
use import::ImportSummary;

use super::{NotStarted, Surroundings, import_with};
use crate::arguments::{Invocation, parse};

/// A card with `DSCF0001.RAF` of 2.5 MB, and the folders of one test run.
struct Folders {
    _temp: TempDir,
    root: PathBuf,
}

impl Folders {
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        // The free space in memory is matched by path, so the paths must be the
        // canonical ones the command resolves.
        let root = fs::canonicalize(temp.path()).unwrap();
        fs::create_dir_all(root.join("card/DCIM/100_FUJI")).unwrap();
        fs::write(
            root.join("card/DCIM/100_FUJI/DSCF0001.RAF"),
            vec![0_u8; 2_500_000],
        )
        .unwrap();
        fs::create_dir(root.join("state")).unwrap();
        Folders { _temp: temp, root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// Runs `framehive import <card> <arguments>` against `free_space`.
    fn import(&self, arguments: &[&Path], free_space: &MemoryFreeSpace) -> Run {
        let mut line: Vec<OsString> = vec!["framehive".into(), "import".into()];
        line.push(self.path("card").into());
        line.extend(
            arguments
                .iter()
                .map(|argument| argument.as_os_str().to_owned()),
        );
        let Ok(Invocation::Import(arguments)) = parse(line) else {
            panic!("not an import");
        };
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let result = import_with(
            &arguments,
            None,
            Surroundings {
                state_folder: &self.path("state"),
                free_space,
                output: &mut output,
                errors: &mut errors,
            },
        );
        Run {
            result,
            output: String::from_utf8(output).unwrap(),
            errors: String::from_utf8(errors).unwrap(),
        }
    }
}

/// What one run of the command did.
struct Run {
    result: Result<ImportSummary, NotStarted>,
    output: String,
    errors: String,
}

impl Run {
    /// The lines of the error that kept the import from starting.
    fn not_started(&self) -> &[String] {
        match &self.result {
            Err(NotStarted(lines)) => lines,
            Ok(_) => panic!("the import ran: {}", self.output),
        }
    }
}

/// spec: CLI-011.1
#[test]
fn a_short_destination_stops_before_anything_is_created() {
    let folders = Folders::new();
    let library = folders.path("library");
    let free_space = MemoryFreeSpace::unlimited().with_file_system(&library, 1_000_000);

    let run = folders.import(&[Path::new(&library)], &free_space);

    assert_eq!(
        run.not_started(),
        [format!(
            "not enough space at {}: needs 2.5 MB, 1.0 MB free",
            library.display()
        )]
    );
    assert!(!library.exists());
}

/// spec: CLI-011.2
#[test]
fn destinations_on_one_file_system_are_named_together_as_given() {
    let folders = Folders::new();
    let disk = folders.path("disk");
    let free_space = MemoryFreeSpace::unlimited().with_file_system(&disk, 4_000_000);
    // The backup is given with `..`, and named as given.
    fs::create_dir(&disk).unwrap();
    let backup = disk.join("../disk/backup");

    let run = folders.import(
        &[&disk.join("library"), Path::new("--backup"), &backup],
        &free_space,
    );

    assert_eq!(
        run.not_started(),
        [format!(
            "not enough space at {}, {}: needs 5.0 MB, 4.0 MB free",
            disk.join("library").display(),
            backup.display()
        )]
    );
    assert!(!disk.join("library").exists());
    assert!(!disk.join("backup").exists());
}

/// spec: CLI-011.3
#[test]
fn a_dry_run_lists_the_plan_and_reports_the_shortage() {
    let folders = Folders::new();
    let library = folders.path("library");
    let free_space = MemoryFreeSpace::unlimited().with_file_system(&library, 1_000_000);

    let run = folders.import(&[&library, Path::new("--dry-run")], &free_space);

    assert_eq!(
        run.not_started(),
        [format!(
            "not enough space at {}: needs 2.5 MB, 1.0 MB free",
            library.display()
        )]
    );
    assert!(
        run.output
            .contains("would copy  DCIM/100_FUJI/DSCF0001.RAF -> "),
        "{}",
        run.output
    );
    assert!(!library.exists());
}

/// spec: CLI-011.4
#[test]
fn unknown_free_space_warns_and_the_import_goes_on() {
    let folders = Folders::new();
    let library = folders.path("library");
    let free_space =
        MemoryFreeSpace::unlimited().with_unreadable(&library, "the file system does not say");

    let run = folders.import(&[&library], &free_space);

    assert_eq!(
        run.errors,
        format!(
            "warning: cannot tell the free space at {}: the file system does not say\n",
            library.display()
        )
    );
    let Ok(summary) = &run.result else {
        panic!("the import did not run: {}", run.errors);
    };
    assert!(!summary.has_failures(), "{}", run.output);
    assert_eq!(summary.files.len(), 1);
    assert!(
        run.output
            .contains("copied  DCIM/100_FUJI/DSCF0001.RAF -> ")
    );
}
