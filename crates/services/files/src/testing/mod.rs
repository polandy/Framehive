// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Test doubles and conformance suites for the ports of this crate (ADR 0020 §4).
//!
//! Adapters run the conformance suites in their tests: `adapter-fs` runs
//! [`check_file_system`] in a [`TempDir`], `adapter-blake3` runs [`check_hasher`] with
//! the reference checksums of BLAKE3.

mod file_system_conformance;
mod fnv;
mod hasher_conformance;
mod memory;
mod temp_dir;

pub use file_system_conformance::{FileSystemUnderTest, MakeSymbolicLink, check_file_system};
pub use fnv::FnvHasher;
pub use hasher_conformance::{HasherUnderTest, check_hasher};
pub use memory::{Fault, MemoryFileSystem};
pub use temp_dir::TempDir;
