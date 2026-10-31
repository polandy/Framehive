// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The files service of Framehive: ports for the file system and for hashing
//! (ADR 0020).
//!
//! Every write of an imported file goes through this service. It reaches storage
//! devices only through the [`FileSystem`] port and computes checksums only through
//! the [`Hasher`] port; `adapter-fs` and `adapter-blake3` implement them.

#![forbid(unsafe_code)]

mod copy;
mod copy_error;
mod ports;
pub mod testing;

pub use copy::copy_verified;
pub use copy_error::{CleanupFailure, CopyError, CopyErrorKind, CopyStep};
pub use ports::{EntryKind, FileSystem, HashState, Hasher, ReadSeek, WriteFile};
