// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The case of the conformance suite for `open_read_from_medium` (ADR 0047).

use std::io::{self, Read, Write};

use super::{Case, CaseResult, ensure, expect_error_kind, failed};

/// The read-back of a verified copy: the synced file reads in full, a missing one is
/// `NotFound` (ADR 0047).
pub(super) fn open_read_from_medium(case: &Case<'_>) -> CaseResult {
    let path = case.path("photo.raf");
    let content: Vec<u8> = (0..=255).cycle().take(100_000).collect();
    let mut file = case
        .fs()
        .create_file(&path)
        .map_err(failed("create_file"))?;
    file.write_all(&content).map_err(failed("write"))?;
    file.sync().map_err(failed("sync"))?;
    drop(file);
    let mut read = Vec::new();
    case.fs()
        .open_read_from_medium(&path)
        .map_err(failed("open_read_from_medium"))?
        .read_to_end(&mut read)
        .map_err(failed("read"))?;
    ensure(
        read == content,
        format!("read {} bytes, expected {}", read.len(), content.len()),
    )?;
    expect_error_kind(
        case.fs().open_read_from_medium(&case.path("missing.raf")),
        io::ErrorKind::NotFound,
        "opening a missing file",
    )
}
