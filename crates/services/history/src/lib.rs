// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The import history of Framehive (IMPORT-009, ADR 0031).
//!
//! The history records every verified copy of an import run, so that a later run
//! recognizes a file by its content wherever the library keeps it now. This service
//! owns the records and the [`ImportHistory`] port; `adapter-sqlite` stores them. It
//! is not part of the import module because the library module needs the same
//! history (ADR 0031, section 7). The import log (IMPORT-010) reads the recorded runs
//! and copies back.

#![forbid(unsafe_code)]

mod log;
mod port;
mod records;
pub mod testing;

pub use log::{LoggedRun, RunLog};
pub use port::{HistoryError, ImportHistory};
pub use records::{CopyKey, CopyRecord, FoundCopy, RecordedTime, RunId, RunStart};
