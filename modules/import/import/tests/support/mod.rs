// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Planned files, a resolver run and expected outcomes shared by the conflict tests
//! (IMPORT-003).

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]
#![allow(dead_code, reason = "each test binary uses only some of the helpers")]

use std::path::Path;

use domain::Timestamp;
use files::testing::{FnvHasher, MemoryFileSystem};
use import::{ConflictOutcome, ConflictPolicy, TargetRequest, TargetResolver};

pub(crate) const SOURCE: &str = "/media/card";
pub(crate) const DESTINATION: &str = "/library";

pub(crate) const POLICIES: [ConflictPolicy; 3] = [
    ConflictPolicy::AddSuffix,
    ConflictPolicy::Skip,
    ConflictPolicy::Fail,
];

pub(crate) fn add(file_system: &MemoryFileSystem, root: &str, relative: &str, content: &[u8]) {
    file_system.add_file(
        &Path::new(root).join(relative),
        content,
        Timestamp::UNIX_EPOCH,
    );
}

/// A source file and its planned target.
pub(crate) struct Planned {
    pub(crate) source_path: &'static str,
    pub(crate) target: &'static str,
    pub(crate) size: u64,
}

pub(crate) fn planned(
    file_system: &MemoryFileSystem,
    source_path: &'static str,
    content: &[u8],
    target: &'static str,
) -> Planned {
    add(file_system, SOURCE, source_path, content);
    Planned {
        source_path,
        target,
        size: content.len() as u64,
    }
}

pub(crate) fn resolve_all(
    file_system: &MemoryFileSystem,
    policy: ConflictPolicy,
    files: &[Planned],
) -> Vec<ConflictOutcome> {
    let mut resolver = TargetResolver::new(
        file_system,
        &FnvHasher,
        Path::new(SOURCE),
        Path::new(DESTINATION),
        policy,
    );
    files
        .iter()
        .map(|file| {
            resolver
                .resolve(&TargetRequest {
                    source_path: Path::new(file.source_path),
                    size: file.size,
                    target: file.target,
                })
                .unwrap()
        })
        .collect()
}

pub(crate) fn copy_to(target: &str) -> ConflictOutcome {
    ConflictOutcome::CopyTo {
        target: target.to_owned(),
    }
}

pub(crate) fn already_imported(target: &str) -> ConflictOutcome {
    ConflictOutcome::AlreadyImported {
        target: target.to_owned(),
    }
}
