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
mod template;
#[cfg(test)]
mod tests;

use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use adapter_blake3::Blake3Hasher;
use adapter_clock::SystemClock;
use adapter_fs::StdFileSystem;
use adapter_mounts::MountsDeviceSource;
use devices::DeviceSource;
use domain::Clock;
use files::{FileSystem, Hasher};

use crate::{NOT_STARTED, report_error};
use dialog::Dialog;
use events::StdinEvents;
use state::state_file;

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
    let ports = Ports {
        file_system: &file_system,
        hasher: &Blake3Hasher,
        clock: &clock,
        devices: &devices,
    };
    let mut events = StdinEvents::start(POLL_INTERVAL);
    let mut output = io::stdout().lock();
    let mut warnings = io::stderr();
    Dialog::start(ports, environment, &mut events, &mut output, &mut warnings).run();
    ExitCode::SUCCESS
}

/// Finds the home folder, the current folder and the state file.
///
/// # Errors
///
/// Returns the message when the home folder or the current folder is unknown.
fn environment() -> Result<Environment, String> {
    let home = std::env::home_dir()
        .filter(|home| home.is_absolute())
        .ok_or_else(|| "the home folder is unknown; set HOME".to_owned())?;
    let current_folder = std::env::current_dir()
        .map_err(|error| format!("the current folder cannot be read: {error}"))?;
    let state_file = state_file(std::env::var_os("XDG_STATE_HOME").as_deref(), &home);
    Ok(Environment {
        home,
        current_folder,
        state_file,
    })
}
