// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The target names that the import jobs of one application reserved when they were
//! enqueued (IMPORT-015, "Reserved target names", ADR 0036 §6).

use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::conflicts::caseless_key;

/// The register of reserved target names that the import jobs of one application
/// share, like the session counter. It lives in the process only.
#[derive(Default)]
pub struct ReservedTargetNames {
    register: Mutex<Register>,
}

/// Which job reserved each name, by the canonical caseless key of its absolute path.
#[derive(Default)]
struct Register {
    next_holder: u64,
    names: HashMap<String, u64>,
}

/// The names one job reserved; dropping it releases them.
pub(crate) struct ReservedNames {
    register: Arc<ReservedTargetNames>,
    holder: u64,
}

impl ReservedTargetNames {
    /// An empty register.
    #[must_use]
    pub fn new() -> Self {
        ReservedTargetNames::default()
    }

    /// Reserves `targets`, absolute paths, for one job until the returned names are
    /// dropped. A name another job reserved stays that job's.
    pub(crate) fn reserve<'t>(
        self: &Arc<Self>,
        targets: impl IntoIterator<Item = &'t Path>,
    ) -> ReservedNames {
        let mut register = self.register();
        let holder = register.next_holder;
        register.next_holder += 1;
        for target in targets {
            register.names.entry(key(target)).or_insert(holder);
        }
        ReservedNames {
            register: Arc::clone(self),
            holder,
        }
    }

    /// Whether a job other than the one holding `own` reserved `target`.
    pub(crate) fn taken(&self, target: &Path, own: Option<&ReservedNames>) -> bool {
        let own = own.map(|names| names.holder);
        self.register()
            .names
            .get(&key(target))
            .is_some_and(|holder| Some(*holder) != own)
    }

    /// The register, also after a thread panicked while it held it: every change to it
    /// is complete when the lock is released.
    fn register(&self) -> MutexGuard<'_, Register> {
        self.register.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl fmt::Debug for ReservedTargetNames {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.register().names.len();
        formatter
            .debug_struct("ReservedTargetNames")
            .field("names", &count)
            .finish()
    }
}

impl Drop for ReservedNames {
    fn drop(&mut self) {
        let holder = self.holder;
        self.register
            .register()
            .names
            .retain(|_, reserved_by| *reserved_by != holder);
    }
}

/// The key of `target`, so that names a file system may not tell apart are one name
/// (ADR 0026).
fn key(target: &Path) -> String {
    caseless_key(&target.to_string_lossy())
}
