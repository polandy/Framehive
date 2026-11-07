// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `framehive import` with a free space in memory (CLI-011): the process tests cannot
//! replace the real one.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{fs, io};

use files::testing::{MemoryCameras, MemoryFreeSpace, TempDir};
use files::{FileSystemId, FreeSpace, SpaceOnFileSystem};
use import::ImportSummary;

use super::{NotStarted, Surroundings, exit_code, import_with, locations};
use crate::arguments::{Invocation, parse};

/// A card with `DSCF0001.RAF` of 2.5 MB, and the folders of one test run.
pub(super) struct Folders {
    _temp: TempDir,
    root: PathBuf,
}

impl Folders {
    pub(super) fn new() -> Self {
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

    pub(super) fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// Runs `framehive import <card> --destination <arguments>` against `free_space`.
    fn import(&self, arguments: &[&Path], free_space: &MemoryFreeSpace) -> Run {
        self.import_from(
            self.path("card").as_os_str(),
            arguments,
            Arc::new(free_space.clone()),
            &MemoryCameras::new(),
        )
    }

    /// Runs `framehive import <source> --destination <library> <options>` with `cameras`
    /// connected.
    pub(super) fn import_camera(
        &self,
        source: &str,
        options: &[&str],
        cameras: &MemoryCameras,
    ) -> Run {
        let library = self.path("library");
        let mut arguments = vec![library.as_path()];
        arguments.extend(options.iter().map(Path::new));
        self.import_from(
            source.as_ref(),
            &arguments,
            Arc::new(MemoryFreeSpace::unlimited()),
            cameras,
        )
    }

    /// Runs `framehive import <source> --destination <arguments>` against `free_space`
    /// and `cameras`.
    fn import_from(
        &self,
        source: &OsStr,
        arguments: &[&Path],
        free_space: Arc<dyn FreeSpace + Send + Sync>,
        cameras: &MemoryCameras,
    ) -> Run {
        // The first of `arguments` is the destination.
        let mut line: Vec<OsString> = vec!["framehive".into(), "import".into()];
        line.push(source.to_owned());
        line.push("--destination".into());
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
        // One source: its result is the only one.
        let locations = locations(&arguments).unwrap();
        let result = import_with(
            &arguments,
            &locations,
            None,
            Surroundings {
                state_folder: &self.path("state"),
                free_space,
                cameras,
                output: &mut output,
                errors: &mut errors,
            },
        )
        .and_then(|mut results| results.remove(0));
        Run {
            result,
            output: String::from_utf8(output).unwrap(),
            errors: String::from_utf8(errors).unwrap(),
        }
    }
}

/// What one run of the command did.
pub(super) struct Run {
    pub(super) result: Result<ImportSummary, NotStarted>,
    pub(super) output: String,
    pub(super) errors: String,
}

impl Run {
    /// The lines of the error that kept the import from starting.
    pub(super) fn not_started(&self) -> &[String] {
        match &self.result {
            Err(NotStarted { messages, .. }) => messages,
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

/// One file system with more space than any test writes at the first question, and
/// `later` bytes at every later one: another job filled it meanwhile.
struct FillingFreeSpace {
    asked: AtomicUsize,
    later: u64,
}

impl FreeSpace for FillingFreeSpace {
    fn free_space(&self, _folder: &Path) -> io::Result<SpaceOnFileSystem> {
        let first = self.asked.fetch_add(1, Ordering::SeqCst) == 0;
        Ok(SpaceOnFileSystem {
            file_system: FileSystemId::new(1),
            available_bytes: if first { u64::MAX } else { self.later },
        })
    }
}

/// spec: CLI-011.8
#[test]
fn a_shortage_found_when_the_job_starts_creates_nothing() {
    let folders = Folders::new();
    let library = folders.path("new/library");
    let free_space = FillingFreeSpace {
        asked: AtomicUsize::new(0),
        later: 1_000_000,
    };

    let run = folders.import_from(
        folders.path("card").as_os_str(),
        &[&library],
        Arc::new(free_space),
        &MemoryCameras::new(),
    );

    let line = format!(
        "not enough space at {}: needs 2.5 MB, 1.0 MB free",
        library.display()
    );
    assert_eq!(run.not_started(), std::slice::from_ref(&line));
    assert_eq!(run.errors, format!("error: {line}\n"));
    assert!(!folders.path("new").exists());
    assert_eq!(exit_code(&run.result), 2);
}

/// The folders of `framehive import /card --destination /library --backup backup`,
/// with the backup resolved to `/disk/backup`.
fn folders_with_backup() -> super::folders::Folders {
    let destination = |path: &str| crate::paths::Destination {
        path: PathBuf::from(path),
    };
    super::folders::Folders {
        source: PathBuf::from("/card"),
        source_is_camera: false,
        destination: destination("/library"),
        given_destination: PathBuf::from("/library"),
        backups: vec![(destination("/disk/backup"), PathBuf::from("backup"))],
        video_destination: None,
        video_backups: Vec::new(),
    }
}

fn not_created(destination: &str, folder: &str) -> import::DestinationNotCreated {
    import::DestinationNotCreated {
        destination: PathBuf::from(destination),
        folder: PathBuf::from(folder),
        error: io::Error::other("the disk is read-only"),
    }
}

#[test]
fn a_destination_the_job_cannot_create_is_named_by_its_folder() {
    let message = folders_with_backup().not_created_message(not_created("/library", "/library"));

    assert_eq!(
        message,
        "the destination /library cannot be created: the disk is read-only"
    );
}

#[test]
fn a_backup_destination_the_job_cannot_create_is_named_as_given() {
    let message = folders_with_backup().not_created_message(not_created("/disk/backup", "/disk"));

    assert_eq!(
        message,
        "the backup destination backup cannot be used: \
         the destination /disk cannot be created: the disk is read-only"
    );
}
