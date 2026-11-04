// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The guided import (CLI-002, CLI-003): `framehive` without a subcommand waits for a memory
//! card, shows what it will do, imports the card on Enter and ejects it (ADR 0024).

mod card;
mod dialog;
mod events;
mod format;
mod progress;
mod state;
mod summary;
mod template;
#[cfg(test)]
mod tests;

use std::ffi::OsStr;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use adapter_blake3::Blake3Hasher;
use adapter_clock::SystemClock;
use adapter_fs::StdFileSystem;
use adapter_mounts::MountsDeviceSource;
use adapter_sequence_counters::FileSequenceCounters;
use devices::DeviceSource;
use domain::Clock;
use files::{FileSystem, Hasher};
use import::{ImportRun, SessionSequence};
use naming::SequenceCounters;

use crate::{NOT_STARTED, report_error};
use dialog::Dialog;
use events::StdinEvents;
use state::state_file;
pub(crate) use state::state_folder;

/// How often the devices are polled while the dialog waits for a line (CLI-002).
pub(crate) const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// The ports the dialog works through.
#[derive(Clone, Copy)]
pub(crate) struct Ports<'a> {
    /// The files of the cards, the library and the state file.
    pub(crate) file_system: &'a dyn FileSystem,
    /// The checksums of the verified copy.
    pub(crate) hasher: &'a dyn Hasher,
    /// The time of the import and of the progress line.
    pub(crate) clock: &'a dyn Clock,
    /// The memory cards.
    pub(crate) devices: &'a dyn DeviceSource,
    /// The persisted sequence counters (ADR 0029).
    pub(crate) counters: &'a dyn SequenceCounters,
    /// The session counter, shared by the imports of all cards in this run.
    pub(crate) session: &'a SessionSequence,
}

impl<'a> Ports<'a> {
    /// An import run through these ports.
    pub(crate) fn import_run(&self) -> ImportRun<'a> {
        ImportRun::new(
            self.file_system,
            self.hasher,
            self.clock,
            self.counters,
            self.session,
        )
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
    /// The folder of the state files, which holds the sequence counters (ADR 0029).
    pub(crate) state_folder: PathBuf,
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
    let file_system = StdFileSystem::new();
    let clock = SystemClock::new();
    let devices = MountsDeviceSource::new();
    let counters = FileSequenceCounters::new(environment.state_folder.clone());
    let session = SessionSequence::new();
    let ports = Ports {
        file_system: &file_system,
        hasher: &Blake3Hasher,
        clock: &clock,
        devices: &devices,
        counters: &counters,
        session: &session,
    };
    let mut events = StdinEvents::start(POLL_INTERVAL);
    let mut output = io::stdout().lock();
    let mut warnings = io::stderr();
    Dialog::start(ports, environment, &mut events, &mut output, &mut warnings).run();
    ExitCode::SUCCESS
}

/// Finds the home folder, the current folder and the state file of this process.
///
/// # Errors
///
/// Returns the message when the home folder or the current folder is unknown.
fn environment() -> Result<Environment, String> {
    environment_from(
        std::env::home_dir(),
        std::env::current_dir(),
        std::env::var_os("XDG_STATE_HOME").as_deref(),
    )
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
    })
}
