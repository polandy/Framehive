// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The progress of a guided import: one line redrawn after each file, and a stop when
//! the card is pulled (CLI-003, "Import and progress").

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use devices::{Device, DeviceChanges, DeviceSource, DeviceWatch};
use domain::{Clock, Timestamp};
use import::{FileReport, ProgressSink, SourceScan};

use super::POLL_INTERVAL;
use super::format::{Progress, progress_line};

/// A [`ProgressSink`] that redraws the progress line and polls the devices between
/// files, stopping the run when the card is gone.
pub(super) struct ImportProgress<'s> {
    output: &'s mut dyn Write,
    watch: &'s mut DeviceWatch,
    devices: &'s dyn DeviceSource,
    clock: &'s dyn Clock,
    card: &'s Device,
    /// The sizes of the planned files, in plan order.
    sizes: Vec<u64>,
    progress: Progress,
    started: Timestamp,
    last_poll: Timestamp,
    /// The width of the line drawn last, to blank what a shorter line leaves.
    width: usize,
    /// The device changes seen during the run, to be printed after it.
    changes: DeviceChanges,
    card_gone: bool,
}

/// The parts of the dialog an import works with.
pub(super) struct ImportContext<'s> {
    /// Where the progress line is drawn.
    pub(super) output: &'s mut dyn Write,
    /// The watch the dialog polls the devices with.
    pub(super) watch: &'s mut DeviceWatch,
    /// The devices.
    pub(super) devices: &'s dyn DeviceSource,
    /// The clock of the rate and of the polls.
    pub(super) clock: &'s dyn Clock,
}

impl<'s> ImportProgress<'s> {
    /// Starts the progress of importing files of `sizes` from `card`, and draws the
    /// first line.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the context hands its borrows over to the progress"
    )]
    pub(super) fn start(context: ImportContext<'s>, card: &'s Device, sizes: Vec<u64>) -> Self {
        let now = context.clock.now();
        let mut progress = ImportProgress {
            output: context.output,
            watch: context.watch,
            devices: context.devices,
            clock: context.clock,
            card,
            progress: Progress {
                total_files: sizes.len(),
                total_bytes: sizes.iter().sum(),
                ..Progress::default()
            },
            sizes,
            started: now,
            last_poll: now,
            width: 0,
            changes: DeviceChanges::default(),
            card_gone: false,
        };
        progress.draw();
        progress
    }

    /// Ends the progress line and returns the device changes seen during the run.
    pub(super) fn finish(self) -> DeviceChanges {
        let _ = writeln!(self.output);
        let _ = self.output.flush();
        self.changes
    }

    fn draw(&mut self) {
        let line = progress_line(&self.progress);
        let width = line.chars().count();
        let blank = " ".repeat(self.width.saturating_sub(width));
        let _ = write!(self.output, "\r{line}{blank}");
        let _ = self.output.flush();
        self.width = width;
    }

    /// Polls the devices once a poll interval has passed since the last poll.
    fn poll_when_due(&mut self) {
        let now = self.clock.now();
        if elapsed(self.last_poll, now) < POLL_INTERVAL {
            return;
        }
        self.last_poll = now;
        // A failed poll says nothing about the card; the next one may succeed.
        let Ok(changes) = self.watch.poll(self.devices) else {
            return;
        };
        if changes.removed.contains(self.card) {
            self.card_gone = true;
        }
        self.changes.removed.extend(changes.removed);
        self.changes.added.extend(changes.added);
    }
}

impl ProgressSink for ImportProgress<'_> {
    fn scan_finished(&mut self, _scan: &SourceScan) {}

    fn file_started(&mut self, _path: &Path) {}

    fn file_finished(&mut self, _report: &FileReport) {
        let size = self
            .sizes
            .get(self.progress.done_files)
            .copied()
            .unwrap_or(0);
        self.progress.done_files += 1;
        self.progress.done_bytes += size;
        self.progress.elapsed = elapsed(self.started, self.clock.now());
        self.draw();
    }

    fn should_continue(&mut self) -> bool {
        self.poll_when_due();
        !self.card_gone
    }
}

/// The time from `from` to `to`, or zero if `to` is earlier.
fn elapsed(from: Timestamp, to: Timestamp) -> Duration {
    let nanoseconds = |time: Timestamp| {
        i128::from(time.unix_seconds()) * 1_000_000_000 + i128::from(time.subsec_nanoseconds())
    };
    let difference = (nanoseconds(to) - nanoseconds(from)).max(0);
    let seconds = u64::try_from(difference / 1_000_000_000).unwrap_or(u64::MAX);
    let rest = u32::try_from(difference % 1_000_000_000).unwrap_or(0);
    Duration::new(seconds, rest)
}
