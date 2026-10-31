// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The metadata service of Framehive.
//!
//! It reads the capture time from a file's EXIF with its own parser (CORE-005,
//! ADR 0021): JPEG and MPO, TIFF-based RAW files in either byte order, and RAF. The
//! parser checks every offset against the file size, reads each IFD at most once and
//! caps the bytes it reads, so a damaged file gives a reason and never a panic.

#![forbid(unsafe_code)]

mod capture_time;
mod jpeg;
mod raf;
mod reason;
mod source;
pub mod testing;
mod tiff;
mod values;

pub use capture_time::capture_time_from_bytes;
pub use reason::NoCaptureTime;
