// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The libgphoto2 adapter of Framehive: [`Gphoto2Cameras`] implements the
//! [`Cameras`](files::Cameras) port of `files` with libgphoto2, for cameras and phones
//! connected over PTP or MTP (IMPORT-014, ADR 0034). Only apps depend on it. See
//! `README.md`.

mod cameras;
mod ffi;
mod file_system;
mod library;
mod reader;
mod session;

pub use cameras::Gphoto2Cameras;
