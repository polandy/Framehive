// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The metadata service of Framehive.
//!
//! It reads the capture time and the camera model from a file's EXIF with its own parser (CORE-005,
//! ADR 0021): JPEG and MPO, TIFF-based RAW files in either byte order, and RAF. The
//! parser checks every offset against the file size, reads each IFD at most once and
//! caps the bytes it reads, so a damaged file gives a reason and never a panic.
//!
//! It reads and writes XMP packets in [`xmp`] (CORE-009, ADR 0039).

#![forbid(unsafe_code)]

mod camera_model;
mod capture_time;
mod file;
mod jpeg;
mod raf;
mod reason;
mod source;
pub mod testing;
mod tiff;
mod values;
pub mod xmp;

pub use camera_model::camera_model_from_bytes;
pub use capture_time::capture_time_from_bytes;
pub use file::{CaptureTimeError, read_camera_model, read_capture_time};
pub use reason::NoCaptureTime;
