// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The dialog of the guided import (CLI-002, CLI-014, ADR 0024): prompts, waiting for a
//! card or camera and the remembered answers. The device itself is handled in
//! `card.rs`.

use std::io::Write;
use std::path::{Path, PathBuf};

use devices::{Device, DeviceChanges, DeviceWatch, PollError};
use import::JobCode;
use jobs::JobQueue;

use super::events::{Event, EventSource};
use super::source::Source;
use super::state::{RememberedState, load_state, save_state};
use super::{Environment, Ports};

/// The question of the `b` choice at the plan summary (CLI-006).
const BACKUP_PROMPT: &str =
    "Backup folder to add, or the number of one to remove [Enter keeps them]: ";

/// Why a prompt did not return an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stop {
    /// The person typed `q` or the input ended: the dialog ends.
    Quit,
    /// The card or camera the prompt is about went: the dialog waits again.
    CardPulled,
}

/// A card pulled or a camera disconnected during its import, to be imported again when
/// it comes back (CLI-003, "A card pulled during the import"; CLI-014).
#[derive(Debug, Clone)]
pub(super) struct Interrupted {
    /// The name of the card, or the model of the camera.
    pub(super) name: String,
    /// Whether the device is a camera.
    pub(super) camera: bool,
    /// The job code of the interrupted import.
    pub(super) job_code: Option<JobCode>,
}

/// The guided import: its ports, its input and output and what it knows.
pub(crate) struct Dialog<'a> {
    pub(super) ports: Ports<'a>,
    pub(super) environment: Environment,
    events: &'a mut dyn EventSource,
    pub(super) output: &'a mut dyn Write,
    warnings: &'a mut dyn Write,
    pub(super) state: RememberedState,
    pub(super) watch: DeviceWatch,
    /// The present cards and cameras that may be offered for import, in the order they
    /// appeared.
    offerable: Vec<Device>,
    /// A card that was ejected, whose removal is not reported.
    pub(super) ejected: Option<Device>,
    /// Cameras reported as removed when they stopped answering, whose removal is not
    /// reported again (CLI-014).
    pub(super) unreported: Vec<Device>,
    /// Cameras kept open after their import, until they are disconnected (CLI-014).
    pub(super) kept_open: Vec<Source>,
    /// Whether the last listing of the cards failed, so that a lasting failure is
    /// reported once.
    cards_failed: bool,
    /// The same for the cameras.
    cameras_failed: bool,
    /// A card pulled or a camera disconnected during its import.
    pub(super) interrupted: Option<Interrupted>,
    /// The job queue the imports run in (IMPORT-015); the dialog follows each import
    /// job until it ends. Nothing reads its updates: each import job's own reports say
    /// how it ended.
    pub(super) jobs: JobQueue,
}

/// Which card to import, and how.
enum Chosen {
    /// A card offered after it appeared.
    New(Device),
    /// The card of an interrupted import, back again.
    Resumed(Device, Interrupted),
}

impl<'a> Dialog<'a> {
    /// Reads the remembered state, printing a warning for each problem with it.
    pub(crate) fn start(
        ports: Ports<'a>,
        environment: Environment,
        events: &'a mut dyn EventSource,
        output: &'a mut dyn Write,
        warnings: &'a mut dyn Write,
    ) -> Self {
        let loaded = load_state(ports.file_system(), &environment.state_file);
        let mut dialog = Dialog {
            ports,
            environment,
            events,
            output,
            warnings,
            state: loaded.state,
            watch: DeviceWatch::new(),
            offerable: Vec::new(),
            ejected: None,
            unreported: Vec::new(),
            kept_open: Vec::new(),
            cards_failed: false,
            cameras_failed: false,
            interrupted: None,
            jobs: JobQueue::start().0,
        };
        for warning in &loaded.warnings {
            dialog.warn(warning);
        }
        dialog
    }

    /// Runs the dialog until the person quits.
    pub(crate) fn run(&mut self) {
        self.say("Framehive guided import. Type q and Enter to quit.");
        let _ = self.import_cards();
    }

    fn import_cards(&mut self) -> Result<(), Stop> {
        if self.state.destination.is_none() {
            let default = self.environment.home.join("Pictures");
            self.ask_destination(&default)?;
        }
        loop {
            let outcome = match self.wait_for_card()? {
                Chosen::New(card) => self.import_device(&card, None),
                Chosen::Resumed(card, interrupted) => self.import_device(&card, Some(interrupted)),
            };
            match outcome {
                Err(Stop::Quit) => return Err(Stop::Quit),
                Ok(()) | Err(Stop::CardPulled) => {}
            }
        }
    }

    /// Waits until a card or camera is offered, or the interrupted one is back, and
    /// returns it.
    fn wait_for_card(&mut self) -> Result<Chosen, Stop> {
        if self.interrupted.is_none() {
            self.say("Waiting for a memory card or camera…");
        }
        loop {
            self.poll();
            if let Some(interrupted) = self.interrupted.clone() {
                let back = |card: &Device| {
                    card.name() == interrupted.name && card.is_camera() == interrupted.camera
                };
                if let Some(card) = self.take_offerable(back) {
                    self.interrupted = None;
                    return Ok(Chosen::Resumed(card, interrupted));
                }
            } else if let Some(card) = self.choose_offered()? {
                return Ok(Chosen::New(card));
            }
            match self.events.next_event() {
                Event::Tick => {}
                Event::End => return Err(self.end_of_input()),
                Event::Line(line) if line.trim() == "q" => return Err(Stop::Quit),
                Event::Line(_) => {
                    self.say("Insert a memory card or connect a camera, or type q to quit.");
                }
            }
        }
    }

    /// The single offered device, or the one the person chooses among several, cards
    /// first.
    fn choose_offered(&mut self) -> Result<Option<Device>, Stop> {
        let mut offered = self.offerable.clone();
        match offered.len() {
            0 => return Ok(None),
            1 => {}
            _ => {
                offered.sort();
                self.say("Which card or camera?");
                for (index, card) in offered.iter().enumerate() {
                    self.say(&format!("  {}  {card}", index + 1));
                }
                let prompt = format!("Choose 1–{} [1]: ", offered.len());
                let index = self.ask_number(&prompt, offered.len())?;
                offered.swap(0, index);
            }
        }
        let chosen = offered.swap_remove(0);
        self.offerable.retain(|card| *card != chosen);
        Ok(Some(chosen))
    }

    /// Asks for a number from 1 to `count`, with 1 as default; returns its index.
    fn ask_number(&mut self, prompt: &str, count: usize) -> Result<usize, Stop> {
        loop {
            let answer = self.prompt(prompt, None)?;
            if answer.is_empty() {
                return Ok(0);
            }
            match answer.parse::<usize>() {
                Ok(number) if (1..=count).contains(&number) => return Ok(number - 1),
                _ => self.say(&format!("Choose a number from 1 to {count}.")),
            }
        }
    }

    /// Removes and returns the first offerable card that `matches`.
    fn take_offerable(&mut self, matches: impl Fn(&Device) -> bool) -> Option<Device> {
        let index = self.offerable.iter().position(matches)?;
        Some(self.offerable.remove(index))
    }

    /// Asks where the photos go, with `default` for an empty answer, and remembers the
    /// answer (CLI-002, "Destination").
    pub(super) fn ask_destination(&mut self, default: &Path) -> Result<(), Stop> {
        let prompt = format!("Where should the photos go? [{}]: ", default.display());
        let answer = self.prompt(&prompt, None)?;
        let destination = if answer.is_empty() {
            default.to_path_buf()
        } else {
            resolve_answer(&answer, &self.environment)
        };
        self.state.destination = Some(destination);
        self.remember();
        Ok(())
    }

    /// Lists the backup destinations and asks for one to add or remove, and remembers
    /// a change (CLI-006, "Choosing backup destinations"). An answer of digits is the
    /// number of a backup destination; a folder named by digits is typed as `./<n>`.
    pub(super) fn ask_backups(&mut self) -> Result<(), Stop> {
        for (index, backup) in self.state.backups.clone().iter().enumerate() {
            self.say(&format!("  {}  {}", index + 1, backup.display()));
        }
        loop {
            let answer = self.prompt(BACKUP_PROMPT, None)?;
            if answer.is_empty() {
                return Ok(());
            }
            if !answer.bytes().all(|byte| byte.is_ascii_digit()) {
                let backup = resolve_answer(&answer, &self.environment);
                self.state.backups.push(backup);
                break;
            }
            match answer.parse::<usize>() {
                Ok(number) if (1..=self.state.backups.len()).contains(&number) => {
                    self.state.backups.remove(number - 1);
                    break;
                }
                _ => self.say(&format!("There is no backup folder {answer}.")),
            }
        }
        self.remember();
        Ok(())
    }

    /// Asks `y` or `n`, with yes as default.
    pub(super) fn ask_yes(&mut self, prompt: &str, card: Option<&Device>) -> Result<bool, Stop> {
        loop {
            match self.prompt(prompt, card)?.as_str() {
                "" | "y" | "Y" => return Ok(true),
                "n" | "N" => return Ok(false),
                _ => self.say("Please answer y or n."),
            }
        }
    }

    /// Shows `prompt` and waits for a line, polling the devices meanwhile. Returns the
    /// line without surrounding whitespace.
    ///
    /// # Errors
    ///
    /// [`Stop::Quit`] for `q` or the end of input, [`Stop::CardPulled`] when `card` is
    /// pulled meanwhile.
    pub(super) fn prompt(&mut self, prompt: &str, card: Option<&Device>) -> Result<String, Stop> {
        self.write(prompt);
        loop {
            match self.events.next_event() {
                Event::Line(line) => {
                    let answer = line.trim();
                    if answer == "q" {
                        return Err(Stop::Quit);
                    }
                    return Ok(answer.to_owned());
                }
                Event::End => return Err(self.end_of_input()),
                Event::Tick => {
                    let changes = self.poll_quietly();
                    if changes.is_empty() {
                        continue;
                    }
                    self.write("\n");
                    self.show_changes(&changes);
                    if card.is_some_and(|card| changes.removed.contains(card)) {
                        return Err(Stop::CardPulled);
                    }
                    self.write(prompt);
                }
            }
        }
    }

    /// Ends the open prompt line, so that the shell starts on a line of its own.
    fn end_of_input(&mut self) -> Stop {
        self.write("\n");
        Stop::Quit
    }

    /// Polls the devices and prints what changed.
    pub(super) fn poll(&mut self) {
        let changes = self.poll_quietly();
        self.show_changes(&changes);
    }

    /// Polls the cards and the cameras; a lasting failure to list either is reported
    /// once, and the other is still watched (DEVICES-001).
    fn poll_quietly(&mut self) -> DeviceChanges {
        let poll = self.watch.poll(self.ports.devices, self.ports.cameras);
        let (mut cards_failed, mut cameras_failed) = (false, false);
        for error in &poll.errors {
            let (failed, reported) = match error {
                PollError::Cards(_) => (&mut cards_failed, self.cards_failed),
                PollError::Cameras(_) => (&mut cameras_failed, self.cameras_failed),
            };
            *failed = true;
            if !reported {
                self.warn(&error.to_string());
            }
        }
        self.cards_failed = cards_failed;
        self.cameras_failed = cameras_failed;
        poll.changes
    }

    /// Prints `[REMOVED]` and `[NEW]` lines and updates the cards that may be offered.
    pub(super) fn show_changes(&mut self, changes: &DeviceChanges) {
        for card in &changes.removed {
            self.offerable.retain(|offerable| offerable != card);
            // Dropping a camera kept open releases it.
            self.kept_open.retain(|source| source.device() != card);
            if self.ejected.as_ref() == Some(card) {
                self.ejected = None;
            } else if let Some(index) = self.unreported.iter().position(|gone| gone == card) {
                self.unreported.remove(index);
            } else {
                self.say(&format!("[REMOVED] {}", card.name()));
            }
        }
        for card in &changes.added {
            self.say(&format!("[NEW] {card}"));
            self.offerable.push(card.clone());
        }
    }

    /// Writes the state file; a failure is a warning (CLI-003, "Remembered state").
    pub(super) fn remember(&mut self) {
        let path = self.environment.state_file.clone();
        if let Err(error) = save_state(self.ports.file_system(), &path, &self.state) {
            self.warn(&format!(
                "the answers cannot be remembered in {}: {error}",
                path.display()
            ));
        }
    }

    /// Writes one line of the dialog.
    pub(super) fn say(&mut self, line: &str) {
        self.write(line);
        self.write("\n");
    }

    /// Writes text and shows it at once. A closed terminal leaves nothing to report.
    pub(super) fn write(&mut self, text: &str) {
        let _ = self.output.write_all(text.as_bytes());
        let _ = self.output.flush();
    }

    /// Writes a `warning:` line to standard error.
    pub(super) fn warn(&mut self, message: &str) {
        let _ = writeln!(self.warnings, "warning: {message}");
    }
}

/// The destination an answer names: `~/` is the home folder, and a relative path is
/// taken against the current folder.
pub(super) fn resolve_answer(answer: &str, environment: &Environment) -> PathBuf {
    if answer == "~" {
        return environment.home.clone();
    }
    if let Some(rest) = answer.strip_prefix("~/") {
        return environment.home.join(rest);
    }
    environment.current_folder.join(answer)
}
