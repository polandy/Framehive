// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: [`SqliteImportHistory`] passes the conformance suite of the
//! `history::ImportHistory` port in a temporary state folder (CLAUDE.md, "Adding
//! things").

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;
use std::path::PathBuf;

use adapter_sqlite::SqliteImportHistory;
use history::ImportHistory;
use history::testing::{ImportHistoryUnderTest, check_import_history};
use support::TempDir;

/// Bytes that are not a SQLite database.
const NOT_A_DATABASE: &[u8] = b"this is not a SQLite database, and it must stay as it is";

struct InStateFolder {
    _folder: TempDir,
    state_folder: PathBuf,
    history: SqliteImportHistory,
}

impl InStateFolder {
    fn new() -> Self {
        let folder = TempDir::new();
        // The state folder does not exist before the first run.
        let state_folder = folder.path().join("framehive");
        let history = SqliteImportHistory::new(&state_folder);
        InStateFolder {
            _folder: folder,
            state_folder,
            history,
        }
    }
}

impl ImportHistoryUnderTest for InStateFolder {
    fn history(&self) -> &dyn ImportHistory {
        &self.history
    }

    fn reopen(&mut self) {
        self.history = SqliteImportHistory::new(&self.state_folder);
    }

    fn break_storage(&mut self) -> String {
        fs::create_dir_all(&self.state_folder).unwrap();
        fs::write(self.history.file(), NOT_A_DATABASE).unwrap();
        self.reopen();
        self.history.file().display().to_string()
    }

    fn is_unchanged_since_broken(&self) -> bool {
        fs::read(self.history.file()).unwrap() == NOT_A_DATABASE
    }
}

/// spec: IMPORT-009.1
/// spec: IMPORT-009.15
/// spec: IMPORT-010.1
/// spec: IMPORT-010.2
/// spec: IMPORT-010.3
/// spec: IMPORT-010.4
/// spec: IMPORT-010.5
/// spec: IMPORT-010.6
/// spec: IMPORT-010.8
#[test]
fn conforms_to_the_port() {
    let failures = check_import_history(&mut InStateFolder::new);
    assert!(failures.is_empty(), "{failures:#?}");
}
