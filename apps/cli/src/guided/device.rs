// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Opening and reading a card or camera, and ejecting or releasing it after the import
//! (CLI-003, CLI-014).

use devices::{Device, DeviceKind, EjectError};
use import::{ScanError, SourceScan};

use super::dialog::{Dialog, Stop};
use super::source::Source;
use crate::cameras::{OpenFailure, OpenedCamera, open_listed_camera};

impl Dialog<'_> {
    /// Opens `device` if it is a camera and scans it. A camera that another program
    /// holds or that shows no storage can be tried again (CLI-014). Returns `None`
    /// after saying why the device cannot be read.
    pub(super) fn read_device(
        &mut self,
        device: &Device,
    ) -> Result<Option<(Source, SourceScan)>, Stop> {
        loop {
            let source = match device.kind() {
                DeviceKind::Card { mount_point } => Source::card(device, mount_point),
                DeviceKind::Camera { port } => {
                    match open_listed_camera(self.ports.cameras, device.name(), port) {
                        Ok(camera) => Source::camera(device, camera),
                        Err(OpenFailure::InUse(line)) => {
                            if self.try_again(&line, device)? {
                                continue;
                            }
                            return Ok(None);
                        }
                        Err(OpenFailure::Other(reason)) => {
                            self.say(&format!("Could not open {}: {reason}", device.name()));
                            return Ok(None);
                        }
                    }
                }
            };
            self.say(&format!("Reading {}…", device.name()));
            match source.scan(&self.ports) {
                Ok(scan) => return Ok(Some((source, scan))),
                Err(ScanError::NoStorage) => {
                    let line = source
                        .opened_camera()
                        .map(OpenedCamera::no_storage_message)
                        .unwrap_or_default();
                    // A phone shows its storage only to a new session once unlocked.
                    drop(source);
                    if self.try_again(&line, device)? {
                        continue;
                    }
                    return Ok(None);
                }
                Err(error) => {
                    self.say(&format!("Could not read {}: {error}", device.name()));
                    return Ok(None);
                }
            }
        }
    }

    /// Prints the line of CLI-013 as a sentence and asks whether to try again.
    fn try_again(&mut self, line: &str, device: &Device) -> Result<bool, Stop> {
        self.say(&sentence(line));
        self.ask_yes("Try again? [Y/n]: ", Some(device))
    }

    /// Offers to eject a card or release a camera that is still present (CLI-003,
    /// CLI-014).
    pub(super) fn finish_device(&mut self, source: Source) -> Result<(), Stop> {
        let device = source.device().clone();
        if !self.watch.devices().contains(&device) {
            return Ok(());
        }
        if source.is_camera() {
            self.offer_release(source, &device)
        } else {
            self.offer_eject(&device)
        }
    }

    /// Asks whether to release the camera of `source`; on no, the camera stays open
    /// until it is disconnected or the dialog ends (CLI-014, "Release").
    fn offer_release(&mut self, source: Source, camera: &Device) -> Result<(), Stop> {
        let prompt = format!("Release {}? [Y/n]: ", camera.name());
        if self.ask_yes(&prompt, Some(camera))? {
            drop(source);
            self.say(&format!("{} can be disconnected now.", camera.name()));
        } else {
            self.keep_open(source);
        }
        Ok(())
    }

    /// Asks whether to eject `card` and ejects it (CLI-003, "Report and eject").
    fn offer_eject(&mut self, card: &Device) -> Result<(), Stop> {
        let prompt = format!("Eject {}? [Y/n]: ", card.name());
        if !self.ask_yes(&prompt, Some(card))? {
            return Ok(());
        }
        match self.ports.devices.eject(card) {
            Ok(()) => {
                self.say(&format!("{} can be removed now.", card.name()));
                self.forget_ejected(card);
            }
            Err(error) => {
                let reason = match error {
                    EjectError::Failed { message, .. } => message,
                    error => error.to_string(),
                };
                self.say(&format!("Could not eject {}: {reason}", card.name()));
            }
        }
        Ok(())
    }

    /// Notes that `card` was ejected, so that its removal is not reported, and polls.
    pub(super) fn forget_ejected(&mut self, card: &Device) {
        self.ejected = Some(card.clone());
        self.poll();
        // A card that is still listed is reported when it goes.
        self.ejected = None;
    }

    /// Notes that the camera `card`, which stopped answering, was reported as removed,
    /// so that its removal is not reported again (CLI-014).
    pub(super) fn forget_unanswered(&mut self, card: &Device) {
        self.unreported.push(card.clone());
    }

    /// Keeps the camera of `source` open until it is disconnected or the dialog ends
    /// (CLI-014, "Release").
    pub(super) fn keep_open(&mut self, source: Source) {
        self.kept_open.push(source);
    }
}

/// A line of CLI-013, which starts lower case, as a sentence of the dialog.
fn sentence(line: &str) -> String {
    let mut characters = line.chars();
    let first = characters
        .next()
        .map(|first| first.to_uppercase().collect::<String>());
    format!("{}{}.", first.unwrap_or_default(), characters.as_str())
}
