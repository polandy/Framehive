// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The progress of a guided import: one line redrawn after each file, and a stop when
//! the card is pulled or the camera goes (CLI-003, "Import and progress"; CLI-014).

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use devices::{Device, DeviceChanges, DeviceSource, DeviceWatch};
use domain::{Clock, Timestamp};
use files::{Cameras, ReadOnlyFileSystem};
use import::{FileOutcome, FileReport, ProgressSink, SourceScan};

use super::POLL_INTERVAL;
use super::format::{Progress, progress_line};

/// A [`ProgressSink`] that redraws the progress line and polls the devices between
/// files, stopping the run when the card or camera is gone.
pub(super) struct ImportProgress<'s> {
    output: &'s mut dyn Write,
    watch: &'s mut DeviceWatch,
    devices: &'s dyn DeviceSource,
    cameras: &'s dyn Cameras,
    clock: &'s dyn Clock,
    card: &'s Device,
    /// The file system of the camera being imported, to tell whether it still answers.
    camera: Option<&'s dyn ReadOnlyFileSystem>,
    /// A file of the camera failed: look at once whether the camera is gone.
    check_now: bool,
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
    /// The cards.
    pub(super) devices: &'s dyn DeviceSource,
    /// The cameras.
    pub(super) cameras: &'s dyn Cameras,
    /// The clock of the rate and of the polls.
    pub(super) clock: &'s dyn Clock,
}

impl<'s> ImportProgress<'s> {
    /// Starts the progress of importing files of `sizes` from `card`, read through
    /// `camera` for a camera, and draws the first line.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the context hands its borrows over to the progress"
    )]
    pub(super) fn start(
        context: ImportContext<'s>,
        card: &'s Device,
        camera: Option<&'s dyn ReadOnlyFileSystem>,
        sizes: Vec<u64>,
    ) -> Self {
        let now = context.clock.now();
        let mut progress = ImportProgress {
            output: context.output,
            watch: context.watch,
            devices: context.devices,
            cameras: context.cameras,
            clock: context.clock,
            card,
            camera,
            check_now: false,
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

    /// Polls the devices once a poll interval has passed since the last poll, or at
    /// once after a file of a camera failed.
    fn poll_when_due(&mut self) {
        let now = self.clock.now();
        if !self.check_now && elapsed(self.last_poll, now) < POLL_INTERVAL {
            return;
        }
        self.last_poll = now;
        // A failed listing says nothing about the card; the next one may succeed.
        let changes = self.watch.poll(self.devices, self.cameras).changes;
        if changes.removed.contains(self.card) || self.camera_stopped_answering() {
            self.card_gone = true;
        }
        self.check_now = false;
        self.changes.removed.extend(changes.removed);
        self.changes.added.extend(changes.added);
    }
}

impl ProgressSink for ImportProgress<'_> {
    fn scan_finished(&mut self, _scan: &SourceScan) {}

    fn file_started(&mut self, _path: &Path) {}

    fn file_finished(&mut self, report: &FileReport) {
        if self.camera.is_some() && matches!(report.outcome, FileOutcome::Failed(_)) {
            self.check_now = true;
        }
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

impl ImportProgress<'_> {
    /// Whether a file of the camera failed and the camera no longer shows its root,
    /// although it may still be listed (CLI-014.10).
    fn camera_stopped_answering(&self) -> bool {
        self.check_now
            && self
                .camera
                .is_some_and(|camera| camera.list_folder(Path::new("/")).is_err())
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

/// The arguments of [`ImportProgress::start`].
type ProgressStart<'s> = (
    ImportContext<'s>,
    &'s Device,
    Option<&'s dyn ReadOnlyFileSystem>,
    Vec<u64>,
);

/// The progress of an import job, whose line is drawn once the job asks before its
/// first file or ends: an import that cannot start shows no progress line, as when it
/// ran on the dialog's thread (IMPORT-015, CLI-003).
pub(super) struct LazyProgress<'s> {
    waiting: Option<ProgressStart<'s>>,
    progress: Option<ImportProgress<'s>>,
}

impl<'s> LazyProgress<'s> {
    /// The progress that [`ImportProgress::start`] starts with these arguments.
    pub(super) fn new(
        context: ImportContext<'s>,
        card: &'s Device,
        camera: Option<&'s dyn ReadOnlyFileSystem>,
        sizes: Vec<u64>,
    ) -> Self {
        LazyProgress {
            waiting: Some((context, card, camera, sizes)),
            progress: None,
        }
    }

    /// The progress, started now if it waits.
    fn progress(&mut self) -> Option<&mut ImportProgress<'s>> {
        if let Some((context, card, camera, sizes)) = self.waiting.take() {
            self.progress = Some(ImportProgress::start(context, card, camera, sizes));
        }
        self.progress.as_mut()
    }

    /// Ends the progress line, drawn first if the job reported nothing, and returns
    /// the device changes seen during the run.
    pub(super) fn finish(mut self) -> DeviceChanges {
        self.progress();
        self.progress
            .map(ImportProgress::finish)
            .unwrap_or_default()
    }
}

impl ProgressSink for LazyProgress<'_> {
    fn scan_finished(&mut self, scan: &SourceScan) {
        if let Some(progress) = self.progress() {
            progress.scan_finished(scan);
        }
    }

    fn file_started(&mut self, path: &Path) {
        if let Some(progress) = self.progress() {
            progress.file_started(path);
        }
    }

    fn file_finished(&mut self, report: &FileReport) {
        if let Some(progress) = self.progress() {
            progress.file_finished(report);
        }
    }

    fn should_continue(&mut self) -> bool {
        self.progress().is_none_or(ProgressSink::should_continue)
    }
}
