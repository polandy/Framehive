// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Port conformance test: [`FileSequenceCounters`] passes the conformance suite of the
//! `naming::SequenceCounters` port in a temporary state folder (CLAUDE.md, "Adding
//! things").

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod support;

use std::fs;

use adapter_sequence_counters::FileSequenceCounters;
use naming::SequenceCounters;
use naming::testing::{SequenceCountersUnderTest, check_sequence_counters};
use support::TempDir;

struct InStateFolder {
    folder: TempDir,
    counters: FileSequenceCounters,
}

impl InStateFolder {
    fn new() -> Self {
        let folder = TempDir::new();
        // The state folder does not exist before the first run.
        let counters = FileSequenceCounters::new(folder.path().join("framehive"));
        InStateFolder { folder, counters }
    }
}

impl SequenceCountersUnderTest for InStateFolder {
    fn counters(&self) -> &dyn SequenceCounters {
        &self.counters
    }

    fn reopen(&mut self) {
        self.counters = FileSequenceCounters::new(self.folder.path().join("framehive"));
    }

    fn break_storage(&mut self) -> String {
        let file = self.counters.file();
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "format_version=1\nstored_last=many\n").unwrap();
        file.display().to_string()
    }
}

/// spec: IMPORT-002.8
#[test]
fn conforms_to_the_sequence_counters_port() {
    let failures = check_sequence_counters(&mut InStateFolder::new);
    assert!(failures.is_empty(), "{failures:#?}");
}
