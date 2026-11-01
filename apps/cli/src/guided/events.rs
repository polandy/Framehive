// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The input of the guided import as events (ADR 0024 §3): a line, a tick once a poll
//! interval while no line arrives, and the end of input.

use std::io::{self, BufRead};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

/// One input event of the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    /// A line the person typed, without its line break.
    Line(String),
    /// A poll interval passed without a line.
    Tick,
    /// The input ended (Ctrl+D) or cannot be read.
    End,
}

/// Where the dialog takes its events from.
pub(crate) trait EventSource {
    /// Waits for the next event.
    fn next_event(&mut self) -> Event;
}

/// The events of standard input: a second thread reads the lines and sends them over a
/// channel, and waiting longer than the poll interval for one is a tick.
pub(crate) struct StdinEvents {
    lines: Receiver<io::Result<String>>,
    interval: Duration,
}

impl StdinEvents {
    /// Starts the thread that reads standard input.
    pub(crate) fn start(interval: Duration) -> Self {
        let (sender, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in io::stdin().lock().lines() {
                if sender.send(line).is_err() {
                    return;
                }
            }
        });
        StdinEvents { lines, interval }
    }
}

impl EventSource for StdinEvents {
    fn next_event(&mut self) -> Event {
        match self.lines.recv_timeout(self.interval) {
            Ok(Ok(line)) => Event::Line(line),
            Err(RecvTimeoutError::Timeout) => Event::Tick,
            // A line that is not UTF-8 or a failed read ends the input like Ctrl+D.
            Ok(Err(_)) | Err(RecvTimeoutError::Disconnected) => Event::End,
        }
    }
}
