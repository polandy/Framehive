// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the dry-run import jobs of one command count in memory, so that a dry run of
//! several sources shows what a real run would do (IMPORT-016).

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use naming::{SequenceCounters, SequenceCountersError, StoredCounters};

use crate::reserved_names::ReservedNames;

/// The sequence counters and target names that the dry-run import jobs of one command
/// reserved (glossary: Dry-run memory). Its counters start from the persisted ones and
/// keep every change in memory; the persisted counters are never written.
///
/// The application plans each dry-run job with it as its counters and gives it to the
/// job with [`ImportJob::with_dry_run_memory`](crate::ImportJob::with_dry_run_memory).
pub struct DryRunMemory {
    persisted: Arc<dyn SequenceCounters + Send + Sync>,
    /// The counters as the dry runs changed them; `None` until the first change.
    changed: Mutex<Option<StoredCounters>>,
    /// The target names of the dry-run jobs that ended, kept until the memory is dropped.
    kept_names: Mutex<Vec<ReservedNames>>,
}

impl DryRunMemory {
    /// A memory that starts from the `persisted` counters and has reserved nothing.
    #[must_use]
    pub fn new(persisted: Arc<dyn SequenceCounters + Send + Sync>) -> Self {
        DryRunMemory {
            persisted,
            changed: Mutex::new(None),
            kept_names: Mutex::new(Vec::new()),
        }
    }

    /// Keeps the target names of a dry-run job that ended, because a dry run creates no
    /// file that would keep the next job off them.
    pub(crate) fn keep(&self, names: ReservedNames) {
        self.kept_names
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(names);
    }

    /// The changed counters, also after a thread panicked while it held them: every
    /// change is complete when the lock is released.
    fn changed(&self) -> MutexGuard<'_, Option<StoredCounters>> {
        self.changed.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl SequenceCounters for DryRunMemory {
    fn read(&self) -> Result<StoredCounters, SequenceCountersError> {
        match *self.changed() {
            Some(counters) => Ok(counters),
            None => self.persisted.read(),
        }
    }

    fn update(
        &self,
        change: &mut dyn FnMut(StoredCounters) -> Option<StoredCounters>,
    ) -> Result<StoredCounters, SequenceCountersError> {
        let mut changed = self.changed();
        let current = match *changed {
            Some(counters) => counters,
            None => self.persisted.read()?,
        };
        if let Some(next) = change(current) {
            *changed = Some(next);
        }
        Ok(current)
    }
}

impl fmt::Debug for DryRunMemory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DryRunMemory")
            .field("changed", &*self.changed())
            .finish_non_exhaustive()
    }
}
