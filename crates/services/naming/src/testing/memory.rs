// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! [`SequenceCounters`] in memory.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::{SequenceCounters, SequenceCountersError, StoredCounters};

/// [`SequenceCounters`] in memory: tests set the stored values and break the storage.
///
/// ```
/// use naming::testing::MemorySequenceCounters;
/// use naming::{SequenceCounters, StoredCounters};
///
/// let counters = MemorySequenceCounters::new();
/// counters.set(StoredCounters { stored_last: 250, ..StoredCounters::default() });
/// assert_eq!(counters.read()?.stored_last, 250);
/// # Ok::<(), naming::SequenceCountersError>(())
/// ```
#[derive(Debug, Default)]
pub struct MemorySequenceCounters {
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    stored: StoredCounters,
    broken: Option<SequenceCountersError>,
}

impl MemorySequenceCounters {
    /// Counters with every value at 0, as on the first run.
    #[must_use]
    pub fn new() -> Self {
        MemorySequenceCounters::default()
    }

    /// Replaces the stored values.
    pub fn set(&self, stored: StoredCounters) {
        self.state().stored = stored;
    }

    /// The stored values, even when the storage is broken.
    #[must_use]
    pub fn stored(&self) -> StoredCounters {
        self.state().stored
    }

    /// Makes every read and update fail from now on, as a broken counter file at
    /// `line` of `file` does.
    pub fn break_at(&self, file: &str, line: usize) {
        self.state().broken = Some(SequenceCountersError::Broken {
            file: PathBuf::from(file),
            line,
            reason: "not a number".to_owned(),
        });
    }

    fn state(&self) -> MutexGuard<'_, State> {
        // A test that panicked while holding the lock leaves consistent values.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl SequenceCounters for MemorySequenceCounters {
    fn read(&self) -> Result<StoredCounters, SequenceCountersError> {
        let state = self.state();
        match &state.broken {
            Some(error) => Err(error.clone()),
            None => Ok(state.stored),
        }
    }

    fn update(
        &self,
        change: &mut dyn FnMut(StoredCounters) -> Option<StoredCounters>,
    ) -> Result<StoredCounters, SequenceCountersError> {
        // The guard is held across `change`, as the adapter holds its lock.
        let mut state = self.state();
        if let Some(error) = &state.broken {
            return Err(error.clone());
        }
        let before = state.stored;
        if let Some(after) = change(before) {
            state.stored = after;
        }
        Ok(before)
    }
}
