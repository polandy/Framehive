// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `macos-trash` adapter of Framehive: it implements the [`Trash`] port of `files`
//! with `-[NSFileManager trashItemAtURL:resultingItemURL:error:]`, so Finder can put a
//! file back (ADR 0043). It is empty on other systems. Only apps depend on it. See
//! `README.md`.
//!
//! [`Trash`]: files::Trash

#[cfg(target_os = "macos")]
mod ffi;
#[cfg(target_os = "macos")]
mod macos_trash;

#[cfg(target_os = "macos")]
pub use macos_trash::MacosTrash;
