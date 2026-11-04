// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The test double `MemoryImportHistory` passes the conformance suite of its own port,
//! so the import tests run against a history that behaves like the real one.

use history::ImportHistory;
use history::testing::{ImportHistoryUnderTest, MemoryImportHistory, check_import_history};

#[derive(Default)]
struct InMemory {
    history: MemoryImportHistory,
    copies_when_broken: usize,
}

impl ImportHistoryUnderTest for InMemory {
    fn history(&self) -> &dyn ImportHistory {
        &self.history
    }

    // The memory outlives a restart of the code that uses it.
    fn reopen(&mut self) {}

    fn break_storage(&mut self) -> String {
        self.copies_when_broken = self.history.copies().len();
        self.history.break_storage();
        "memory".to_owned()
    }

    fn is_unchanged_since_broken(&self) -> bool {
        self.history.copies().len() == self.copies_when_broken
    }
}

#[test]
fn memory_history_conforms_to_the_import_history_port() {
    let failures = check_import_history(&mut InMemory::default);
    assert!(failures.is_empty(), "{failures:#?}");
}
