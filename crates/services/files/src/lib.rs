// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The files service of Framehive: ports for the file system and for hashing
//! (ADR 0020).
//!
//! Every write of an imported file goes through this service. It reaches storage
//! devices only through the [`FileSystem`] port, or the [`ReadOnlyFileSystem`] port
//! for a source it only reads (ADR 0034), and computes checksums only through the
//! [`Hasher`] port; `adapter-fs` and `adapter-blake3` implement them. Files leave the
//! library only through the [`Trash`] port (ADR 0043).

#![forbid(unsafe_code)]

mod cameras;
mod checksum;
mod copy;
mod copy_error;
mod ports;
pub mod testing;
mod trash;

pub use cameras::{Camera, CameraError, Cameras};
pub use checksum::checksum_file;
pub use copy::{VerifiedCopies, copy_verified, copy_verified_to_all};
pub use copy_error::{CleanupFailure, CopyError, CopyErrorKind, CopyStep};
pub use ports::{
    EntryKind, FileSystem, FileSystemId, FolderEntry, FreeSpace, HashState, Hasher,
    ReadOnlyFileSystem, ReadSeek, SpaceOnFileSystem, WriteFile,
};
pub use trash::{Trash, not_a_file};
