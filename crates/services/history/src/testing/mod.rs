// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The test double and the conformance suite of the
//! [`ImportHistory`](crate::ImportHistory) port (ADR 0031).
//!
//! `adapter-sqlite` runs [`check_import_history`] against a temporary state folder.

mod conformance;
mod memory;

pub use conformance::{ImportHistoryUnderTest, check_import_history};
pub use memory::{MemoryImportHistory, RecordedRun};
