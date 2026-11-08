// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The guided import (CLI-002, CLI-003, CLI-014): `framehive` without a subcommand waits for
//! a memory card or camera, shows what it will do, imports it on Enter and ejects or
//! releases it (ADR 0024).

mod card;
mod device;
mod dialog;
mod error_lines;
mod events;
mod format;
mod progress;
mod source;
mod state;
mod summary;
mod template;
#[cfg(test)]
mod tests;

use std::ffi::OsStr;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use adapter_blake3::Blake3Hasher;
use adapter_clock::SystemClock;
use adapter_free_space::StatvfsFreeSpace;
use adapter_fs::StdFileSystem;
use adapter_gphoto2::Gphoto2Cameras;
use adapter_mounts::MountsDeviceSource;
use adapter_sequence_counters::FileSequenceCounters;
use adapter_sqlite::SqliteImportHistory;
use adapter_xmp::QuickXml;
use devices::DeviceSource;
use files::{Cameras, FileSystem};
use import::{ImportMetadata, ImportPorts, ImportRun, ReservedTargetNames, SessionSequence};

use crate::configuration_file::process_configuration;
use crate::paths::{PathError, canonical_folder};
use crate::{NOT_STARTED, report_error};
use dialog::Dialog;
use events::StdinEvents;
pub(crate) use format::space_text;
use state::state_file;
pub(crate) use state::{create_folders, state_folder};

/// How often the cards and cameras are polled while the dialog waits for a line
/// (CLI-002, CLI-014).
pub(crate) const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// The ports the dialog works through.
#[derive(Clone, Copy)]
pub(crate) struct Ports<'a> {
    /// The ports of the imports, shared with the import jobs (IMPORT-015): the files of
    /// the cards, the library and the state file, the checksums, the time, the sequence
    /// counters, the session counter, the import history and the free space.
    pub(crate) import: &'a ImportPorts,
    /// The memory cards.
    pub(crate) devices: &'a dyn DeviceSource,
    /// The cameras and phones connected over PTP or MTP (CLI-014).
    pub(crate) cameras: &'a dyn Cameras,
    /// Resolves a destination to the canonical absolute path that identifies it in the
    /// import history (IMPORT-009).
    pub(crate) resolve_folder: fn(&Path) -> Result<PathBuf, PathError>,
}

impl<'a> Ports<'a> {
    /// An import run through these ports.
    pub(crate) fn import_run(&self) -> ImportRun<'a> {
        self.import.import_run()
    }

    /// The files of the cards, the library and the state file.
    pub(crate) fn file_system(&self) -> &'a dyn FileSystem {
        &*self.import.file_system
    }
}

/// The folders the dialog resolves answers and the state file against.
#[derive(Debug, Clone)]
pub(crate) struct Environment {
    /// The home folder: `~/` in answers and the default destination.
    pub(crate) home: PathBuf,
    /// The folder relative destinations are taken against.
    pub(crate) current_folder: PathBuf,
    /// The file the answers are remembered in (ADR 0025).
    pub(crate) state_file: PathBuf,
    /// The folder of the state files, which holds the sequence counters (ADR 0029) and
    /// the import history (ADR 0031).
    pub(crate) state_folder: PathBuf,
    /// The `metadata` of `config.yaml` (IMPORT-019).
    pub(crate) metadata: ImportMetadata,
}

/// Runs the guided import on the terminal and returns its exit code (CLI-002, CLI-003).
pub(crate) fn run_guided() -> ExitCode {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        report_error(
            "the guided import needs a terminal; use framehive import <source> <destination> in scripts",
        );
        return ExitCode::from(NOT_STARTED);
    }
    let environment = match environment() {
        Ok(environment) => environment,
        Err(message) => {
            report_error(&message);
            return ExitCode::from(NOT_STARTED);
        }
    };
    let devices = MountsDeviceSource::new();
    let cameras = Gphoto2Cameras::new();
    let import = ImportPorts {
        file_system: Arc::new(StdFileSystem::new()),
        hasher: Arc::new(Blake3Hasher),
        clock: Arc::new(SystemClock::new()),
        counters: Arc::new(FileSequenceCounters::new(environment.state_folder.clone())),
        session: Arc::new(SessionSequence::new()),
        history: Arc::new(SqliteImportHistory::new(&environment.state_folder)),
        free_space: Arc::new(StatvfsFreeSpace::new()),
        reserved_names: Arc::new(ReservedTargetNames::new()),
        xml_syntax: Arc::new(QuickXml::new()),
    };
    let ports = Ports {
        import: &import,
        devices: &devices,
        cameras: &cameras,
        resolve_folder: canonical_folder,
    };
    let mut events = StdinEvents::start(POLL_INTERVAL);
    let mut output = io::stdout().lock();
    let mut warnings = io::stderr();
    Dialog::start(ports, environment, &mut events, &mut output, &mut warnings).run();
    ExitCode::SUCCESS
}

/// Finds the home folder, the current folder, the state file and `config.yaml` of this
/// process.
///
/// # Errors
///
/// Returns the message when the home folder or the current folder is unknown, or
/// `config.yaml` cannot be read.
fn environment() -> Result<Environment, String> {
    let mut environment = environment_from(
        std::env::home_dir(),
        std::env::current_dir(),
        std::env::var_os("XDG_STATE_HOME").as_deref(),
    )?;
    environment.metadata = process_configuration()?.metadata.values();
    Ok(environment)
}

/// Builds the environment from what the process reported, so the errors are testable
/// without changing the real home or current folder (CLI-002.15).
///
/// # Errors
///
/// Returns the message when the home folder is missing or relative, or the current
/// folder cannot be read.
pub(crate) fn environment_from(
    home: Option<PathBuf>,
    current_folder: io::Result<PathBuf>,
    xdg_state_home: Option<&OsStr>,
) -> Result<Environment, String> {
    let home = home
        .filter(|home| home.is_absolute())
        .ok_or_else(|| "the home folder is unknown; set HOME".to_owned())?;
    let current_folder =
        current_folder.map_err(|error| format!("the current folder cannot be read: {error}"))?;
    let state_file = state_file(xdg_state_home, &home);
    let state_folder = state_folder(xdg_state_home, &home);
    Ok(Environment {
        home,
        current_folder,
        state_file,
        state_folder,
        metadata: ImportMetadata::default(),
    })
}
