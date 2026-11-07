// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! How an import job sends its steps to its enqueuer, and how it waits for an answer
//! that a stop request can give too (IMPORT-015.9, IMPORT-015.10, ADR 0036 §7).

use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex, PoisonError};

use jobs::JobControl;

use super::Report;
use crate::report::FileReport;
use crate::run::RunSteps;
use crate::scan::SourceScan;

/// Sends the steps of the run to the enqueuer. Nobody listening is not an error: the
/// import still runs.
pub(super) struct SendingSteps<'s> {
    reports: &'s Sender<Report>,
    control: &'s JobControl,
    asks: bool,
    /// The question the job waits on, so that a stop request can answer it.
    open_question: Arc<Mutex<Option<Arc<Question>>>>,
}

impl RunSteps for SendingSteps<'_> {
    fn scan_finished(&mut self, scan: &SourceScan) {
        let _ = self.reports.send(Report::ScanFinished(scan.clone()));
    }

    fn file_started(&mut self, path: &Path) {
        let _ = self.reports.send(Report::FileStarted(path.to_path_buf()));
    }

    fn file_finished(&mut self, report: FileReport) {
        let _ = self.reports.send(Report::FileFinished(report));
    }

    /// The enqueuer is asked first, so that a stop it asked for while it followed
    /// the reports is seen before the next file.
    fn should_continue(&mut self) -> bool {
        (!self.asks || self.enqueuer_goes_on()) && self.control.should_continue()
    }
}

impl<'s> SendingSteps<'s> {
    /// Steps sent on `reports`; a job that `asks` waits for an answer before each file.
    pub(super) fn new(reports: &'s Sender<Report>, control: &'s JobControl, asks: bool) -> Self {
        let steps = SendingSteps {
            reports,
            control,
            asks,
            open_question: Arc::new(Mutex::new(None)),
        };
        if asks {
            steps.answer_no_when_stopped();
        }
        steps
    }

    /// Answers the open question "no" when the job is asked to stop, so that a job
    /// whose enqueuer does not follow its reports does not keep a queue from shutting
    /// down (IMPORT-015.10, ADR 0036 §7).
    fn answer_no_when_stopped(&self) {
        let open_question = Arc::clone(&self.open_question);
        self.control.notify_on_stop(move || {
            if let Some(question) = lock(&open_question).take() {
                question.answer(false);
            }
        });
    }

    /// Asks the enqueuer whether to go on and waits for the answer. An enqueuer that
    /// no longer listens cannot answer; the import goes on. A stop request answers
    /// "no".
    fn enqueuer_goes_on(&self) -> bool {
        let question = Arc::new(Question::default());
        *lock(&self.open_question) = Some(Arc::clone(&question));
        // Checked after the question is open: a stop request before this point finds
        // no question to answer, and is seen here instead.
        if !self.control.should_continue() {
            return false;
        }
        if self
            .reports
            .send(Report::GoOn(Answer(Arc::clone(&question))))
            .is_err()
        {
            return true;
        }
        let go_on = question.wait();
        lock(&self.open_question).take();
        go_on
    }
}

/// One question whether to go on, answered once: by the enqueuer, by a stop request,
/// or with "go on" when the enqueuer drops it unanswered.
#[derive(Debug, Default)]
struct Question {
    answer: Mutex<Option<bool>>,
    answered: Condvar,
}

impl Question {
    /// Gives `go_on` as the answer, unless the question is answered already.
    fn answer(&self, go_on: bool) {
        let mut answer = lock(&self.answer);
        if answer.is_none() {
            *answer = Some(go_on);
            self.answered.notify_all();
        }
    }

    /// Waits for the answer.
    fn wait(&self) -> bool {
        let mut answer = lock(&self.answer);
        loop {
            if let Some(go_on) = *answer {
                return go_on;
            }
            answer = self
                .answered
                .wait(answer)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// The enqueuer's side of a [`Question`]; dropping it unanswered answers "go on".
#[derive(Debug)]
pub(super) struct Answer(Arc<Question>);

impl Answer {
    /// Answers whether the job goes on with the next file.
    pub(super) fn give(self, go_on: bool) {
        self.0.answer(go_on);
    }
}

impl Drop for Answer {
    fn drop(&mut self) {
        self.0.answer(true);
    }
}

/// `mutex`, also after a thread panicked while it held it: the values behind these
/// locks are set in one step.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
