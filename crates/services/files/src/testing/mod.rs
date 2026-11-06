// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Test doubles and conformance suites for the ports of this crate (ADR 0020 §4).
//!
//! Adapters run the conformance suites in their tests: `adapter-fs` runs
//! [`check_file_system`] in a [`TempDir`], which includes
//! [`check_read_only_file_system`], `adapter-blake3` runs [`check_hasher`] with
//! the reference checksums of BLAKE3, `adapter-free-space` runs [`check_free_space`]
//! in a [`TempDir`].

mod file_system_conformance;
mod fnv;
mod free_space_conformance;
mod hasher_conformance;
mod memory;
mod memory_free_space;
mod read_only_file_system_conformance;
mod temp_dir;

pub use file_system_conformance::{FileSystemUnderTest, check_file_system};
pub use fnv::FnvHasher;
pub use free_space_conformance::{FreeSpaceUnderTest, check_free_space};
pub use hasher_conformance::{HasherUnderTest, check_hasher};
pub use memory::{Fault, MemoryFileSystem};
pub use memory_free_space::MemoryFreeSpace;
pub use read_only_file_system_conformance::{
    AddFile, AddFolder, MakeSymbolicLink, ReadOnlyFileSystemUnderTest, check_read_only_file_system,
};
pub use temp_dir::TempDir;
